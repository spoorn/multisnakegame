use bevy::app::AppExit;
use bevy::prelude::*;
use bevy::window::WindowCloseRequested;
use durian::{ClientConfig, PacketManager};
use iyes_loopless::prelude::{AppLooplessStateExt, IntoConditionalSystem};
use iyes_loopless::state::NextState;

use crate::client::resources::{ClientInfo, ClientPacketManager};
use crate::networking::client_packets::{Disconnect, Ready, SnakeMovement, StartNewGame};
use crate::networking::lobby::client_packets::{CancelLobby, CreateLobby, JoinLobby, LeaveLobby};
use crate::networking::lobby::server_packets::{CreateLobbySuccess, CreateLobbySuccessPacketBuilder, LobbyCanceled, LobbyCanceledPacketBuilder, PlayerJoined, PlayerJoinedPacketBuilder, PlayerLeft, PlayerLeftPacketBuilder};
use crate::networking::server_packets::{EatFood, EatFoodPacketBuilder, ReadyAck, ReadyAckPacketBuilder, SnakePositions, SnakePositionsPacketBuilder, SpawnFood, SpawnFoodPacketBuilder, SpawnSnake, SpawnSnakePacketBuilder, SpawnTail, SpawnTailPacketBuilder, StartNewGameAck, StartNewGameAckPacketBuilder};
use crate::state::GameState;

pub struct ClientPlugin {
    pub client_addr: String,
    pub lobby_server_addr: String,
    pub server_addr: String
}

impl Plugin for ClientPlugin {
    
    fn build(&self, app: &mut App) {
        app
            .insert_resource(ClientInfo { client_addr: self.client_addr.to_owned(), lobby_server_addr: self.lobby_server_addr.to_owned(), server_addr: self.server_addr.to_owned() })
            .add_startup_system(setup_client)
            .add_system(wait_for_ready_ack.run_in_state(GameState::PreGame))
            .add_system(exit_system.run_not_in_state(GameState::MainMenu))
            .add_enter_system(GameState::ConnectToServer, init_client);
    }
}

fn init_lobby_client(mut commands: Commands, client_info: Res<ClientInfo>) {
    let mut manager = PacketManager::new();
    manager.register_receive_packet::<CreateLobbySuccess>(CreateLobbySuccessPacketBuilder).unwrap();
    manager.register_receive_packet::<PlayerJoined>(PlayerJoinedPacketBuilder).unwrap();
    manager.register_receive_packet::<PlayerLeft>(PlayerLeftPacketBuilder).unwrap();
    manager.register_receive_packet::<LobbyCanceled>(LobbyCanceledPacketBuilder).unwrap();
    manager.register_send_packet::<CreateLobby>().unwrap();
    manager.register_send_packet::<CancelLobby>().unwrap();
    manager.register_send_packet::<JoinLobby>().unwrap();
    manager.register_send_packet::<LeaveLobby>().unwrap();

    manager.init_client(ClientConfig::new(client_info.client_addr.to_owned(), client_info.lobby_server_addr.to_owned(), 4, 4)).unwrap();

    commands.insert_resource(ClientPacketManager { manager });
}

fn init_client(mut commands: Commands, client_info: Res<ClientInfo>) {
    let mut manager = PacketManager::new();
    manager.register_receive_packet::<StartNewGameAck>(StartNewGameAckPacketBuilder).unwrap();
    manager.register_receive_packet::<SpawnSnake>(SpawnSnakePacketBuilder).unwrap();
    manager.register_receive_packet::<ReadyAck>(ReadyAckPacketBuilder).unwrap();
    manager.register_receive_packet::<SnakePositions>(SnakePositionsPacketBuilder).unwrap();
    manager.register_receive_packet::<SpawnFood>(SpawnFoodPacketBuilder).unwrap();
    manager.register_receive_packet::<EatFood>(EatFoodPacketBuilder).unwrap();
    manager.register_receive_packet::<SpawnTail>(SpawnTailPacketBuilder).unwrap();
    manager.register_send_packet::<StartNewGame>().unwrap();
    manager.register_send_packet::<Disconnect>().unwrap();
    manager.register_send_packet::<SnakeMovement>().unwrap();
    manager.register_send_packet::<Ready>().unwrap();
    manager.init_client(ClientConfig::new(client_info.client_addr.to_owned(), client_info.server_addr.to_owned(), 7, 4)).unwrap();
    
    manager.send(StartNewGame).unwrap();

    // wait for ack
    // TODO: Switch to lobby view
    //manager.received::<StartNewGameAck, StartNewGameAckPacketBuilder>(true).unwrap();
    commands.insert_resource(ClientPacketManager { manager });
    //commands.insert_resource(NextState(GameState::PreGame));
}

fn setup_client(mut commands: Commands) {
    // is_client
    commands.insert_resource::<bool>(true);
}

fn wait_for_ready_ack(mut commands: Commands, mut manager: ResMut<ClientPacketManager>) {
    if let Some(_ready_acked) = manager.received::<ReadyAck, ReadyAckPacketBuilder>(false).unwrap() {
        commands.insert_resource(NextState(GameState::Running));
        return;
    }
}

fn exit_system(mut manager: ResMut<ClientPacketManager>, exit: EventReader<AppExit>, close_window: EventReader<WindowCloseRequested>) {
    if !exit.is_empty() || !close_window.is_empty() {
        manager.manager.send(Disconnect).unwrap();
    }
}