use bevy::prelude::*;
use durian::{PacketManager, ServerConfig};

use crate::lobby::resources::{Lobbies, Lobby, LobbyPacketManager, LobbyServerInfo};
use crate::networking::lobby::client_packets::{CancelLobby, CancelLobbyPacketBuilder, CreateLobby, CreateLobbyPacketBuilder, JoinLobby, JoinLobbyPacketBuilder, LeaveLobby, LeaveLobbyPacketBuilder};
use crate::networking::lobby::server_packets::{CancelLobbyFailed, CancelLobbySuccess, CreateLobbyFailed, CreateLobbySuccess, JoinLobbyFailed, JoinLobbySuccess, LobbyCanceled, PlayerJoined, PlayerLeft};

/// Lobby server

pub struct LobbyServerPlugin;
impl Plugin for LobbyServerPlugin {
    
    fn build(&self, app: &mut App) {
        app.add_startup_system(init_server)
            .add_system(handle_create_lobby)
            .add_system(handle_cancel_lobby)
            .add_system(handle_join_lobby);
    }
}

fn init_server(mut commands: Commands, server_info: Res<LobbyServerInfo>) {
    let mut manager = PacketManager::new();
    manager.register_receive_packet::<CreateLobby>(CreateLobbyPacketBuilder).unwrap();
    manager.register_receive_packet::<CancelLobby>(CancelLobbyPacketBuilder).unwrap();
    manager.register_receive_packet::<JoinLobby>(JoinLobbyPacketBuilder).unwrap();
    manager.register_receive_packet::<LeaveLobby>(LeaveLobbyPacketBuilder).unwrap();
    manager.register_send_packet::<CreateLobbySuccess>().unwrap();
    manager.register_send_packet::<PlayerJoined>().unwrap();
    manager.register_send_packet::<PlayerLeft>().unwrap();
    manager.register_send_packet::<LobbyCanceled>().unwrap();
    
    manager.init_server(ServerConfig::new_listening(server_info.addr.to_string(), 0, 4, 4)).unwrap();
    
    commands.insert_resource(LobbyPacketManager { manager });
}

fn handle_create_lobby(mut manager: ResMut<LobbyPacketManager>, mut lobbies: ResMut<Lobbies>) {
    let create_lobbies = manager.received_all::<CreateLobby, CreateLobbyPacketBuilder>(false).unwrap();
    for (addr, creates) in create_lobbies.iter() {
        if let Some(create) = creates {
            for (i, create) in create.iter().enumerate() {
                let name = &create.name;
                if i > 0 {
                    manager.send_to(addr, CreateLobbyFailed { name: name.clone(), reason: "You cannot create more than one lobby at a time!".to_string() }).unwrap();
                } else if lobbies.lobbies.contains_key(name) {
                    manager.send_to(addr, CreateLobbyFailed { name: name.clone(), reason: "Lobby with that name already exists".to_string() }).unwrap();
                } else {
                    let lobby = Lobby {
                        name: name.clone(),
                        description: create.description.clone(),
                        leader_addr: addr.clone(),
                        players: vec![(addr.clone(), create.player_name.clone())],
                        max_players: create.max_players
                    };
                    lobbies.lobbies.insert(name.clone(), lobby);
                    manager.send_to(addr, CreateLobbySuccess { name: name.clone() }).unwrap();
                }
            }
        }
    }
}

fn handle_cancel_lobby(mut manager: ResMut<LobbyPacketManager>, mut lobbies: ResMut<Lobbies>) {
    let cancel_lobbies = manager.received_all::<CancelLobby, CancelLobbyPacketBuilder>(false).unwrap();
    for (addr, cancels) in cancel_lobbies.iter() {
        if let Some(cancel) = cancels {
            for cancel in cancel.iter() {
                let mut removed = false;
                match lobbies.lobbies.get(&cancel.name) {
                    None => {
                        manager.send_to(addr, CancelLobbyFailed { name: cancel.name.clone(), reason: "Cannot cancel lobby as it doesn't exist!".to_string() }).unwrap();
                    }
                    Some(lobby) => {
                        if lobby.leader_addr == *addr {
                            for (lobby_addr, _player_name) in lobby.players.iter() {
                                // For other players, send LobbyCanceled
                                if lobby_addr != addr {
                                    manager.send_to(lobby_addr, LobbyCanceled { name: lobby.name.clone(), reason: "Lobby was canceled by the leader".to_string() }).unwrap();
                                }
                            }
                            // For leader, send CancelLobbySuccess
                            manager.send_to(addr, CancelLobbySuccess { name: lobby.name.clone() }).unwrap();
                            removed = true;
                        } else {
                            manager.send_to(addr, CancelLobbyFailed { name: lobby.name.clone(), reason: "Cannot cancel lobby if you are not the leader".to_string() }).unwrap();
                        }
                    }
                }
                if removed {
                    lobbies.lobbies.remove(&cancel.name);
                }
            }
        }
    }
}

fn handle_join_lobby(mut manager: ResMut<LobbyPacketManager>, mut lobbies: ResMut<Lobbies>) {
    let join_lobbies = manager.received_all::<JoinLobby, JoinLobbyPacketBuilder>(false).unwrap();
    for (addr, joins) in join_lobbies.iter() {
        if let Some(joins) = joins {
            for (i, join) in joins.iter().enumerate() {
                if i > 0 {
                    manager.send_to(addr, JoinLobbyFailed { name: join.name.clone(), reason: "Cannot join more than one lobby!".to_string() }).unwrap();
                } else {
                    match lobbies.lobbies.get_mut(&join.name) {
                        None => {
                            manager.send_to(addr, JoinLobbyFailed { name: join.name.clone(), reason: "Lobby with this name doesn't exist!".to_string() }).unwrap();
                        }
                        Some(lobby) => {
                            if lobby.players.len() >= lobby.max_players as usize {
                                manager.send_to(addr, JoinLobbyFailed { name: join.name.clone(), reason: "Lobby is full".to_string() }).unwrap();
                            } else {
                                let already_in_lobby = lobby.players.iter().any(|(existing_addr, _player_name)| existing_addr == addr);
                                if already_in_lobby {
                                    manager.send_to(addr, JoinLobbyFailed { name: join.name.clone(), reason: "You are already in the lobby".to_string() }).unwrap();
                                } else {
                                    for (existing_addr, player_name) in lobby.players.iter() {
                                        manager.send_to(existing_addr, PlayerJoined { player_name: player_name.clone(), lobby_name: lobby.name.clone() }).unwrap();
                                    }
                                    manager.send_to(addr, JoinLobbySuccess { name: lobby.name.clone() }).unwrap();
                                    lobby.players.push((addr.to_string(), join.player_name.to_string()));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}