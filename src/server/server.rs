use bevy::app::AppExit;
use bevy::prelude::*;
use iyes_loopless::prelude::{AppLooplessStateExt, IntoConditionalSystem};
use iyes_loopless::state::NextState;

use networking::packet::{PacketManager, ReceiveError};

use crate::networking::client_packets::{Disconnect, DisconnectPacketBuilder, Ready, ReadyPacketBuilder, SnakeMovement, SnakeMovementPacketBuilder, StartNewGame, StartNewGamePacketBuilder};
use crate::networking::server_packets::{EatFood, ReadyAck, SnakePositions, SpawnFood, SpawnSnake, SpawnTail, StartNewGameAck};
use crate::server::resources::{ReadyCount, ServerInfo, ServerPacketManager};
use crate::state::GameState;

pub struct ServerPlugin {
    pub server_addr: String
}

impl Plugin for ServerPlugin {

    fn build(&self, app: &mut App) {
        app.insert_resource(ServerInfo { server_addr: self.server_addr.to_owned(), want_num_clients: 1 })
            .insert_resource(ReadyCount { count: 0 })
            .add_startup_system(setup_packet_manager)
            .add_loopless_state(GameState::MainMenu)
            .add_system(client_disconnect.run_not_in_state(GameState::MainMenu))
            .add_system(wait_for_ready.run_in_state(GameState::PreGame));
    }
}

fn setup_packet_manager(mut commands: Commands, server_info: Res<ServerInfo>) {
    let mut manager = PacketManager::new();
    manager.register_receive_packet::<StartNewGame>(StartNewGamePacketBuilder).unwrap();
    manager.register_receive_packet::<Disconnect>(DisconnectPacketBuilder).unwrap();
    manager.register_receive_packet::<SnakeMovement>(SnakeMovementPacketBuilder).unwrap();
    manager.register_receive_packet::<Ready>(ReadyPacketBuilder).unwrap();
    manager.register_send_packet::<StartNewGameAck>().unwrap();
    manager.register_send_packet::<SpawnSnake>().unwrap();
    manager.register_send_packet::<ReadyAck>().unwrap();
    manager.register_send_packet::<SnakePositions>().unwrap();
    manager.register_send_packet::<SpawnFood>().unwrap();
    manager.register_send_packet::<EatFood>().unwrap();
    manager.register_send_packet::<SpawnTail>().unwrap();
    manager.init_connections(true, 4, 7, server_info.server_addr.to_owned(), None, 1, Some(server_info.want_num_clients as u32)).unwrap();
    
    commands.insert_resource(ServerPacketManager { manager });
}

fn wait_for_ready(mut commands: Commands, mut manager: ResMut<ServerPacketManager>, mut ready_count: ResMut<ReadyCount>, server_info: Res<ServerInfo>) {
    match manager.received_all::<Ready, ReadyPacketBuilder>(false) {
        Ok(readies) => {
            ready_count.count += readies.len() as i32;
            if ready_count.count < 0 {
                panic!("[server] Got more Ready packets than clients!");
            } else if ready_count.count == server_info.want_num_clients as i32 {
                manager.broadcast(ReadyAck).unwrap();
                commands.insert_resource(NextState(GameState::Running));
            }
        }
        Err(e) => {
            panic!("[server] Could not receive Ready packets from clients: {}", e);
        }
    }
}

fn client_disconnect(mut manager: ResMut<ServerPacketManager>, mut exit: EventWriter<AppExit>) {
    let disconnects = manager.manager.received_all::<Disconnect, DisconnectPacketBuilder>(false).unwrap();
    // TODO: Check all clients and only remove the dced one
    for (_addr, disconnect) in disconnects.into_iter() {
        if disconnect.is_some() && disconnect.unwrap().len() > 0 {
            exit.send(AppExit);
            break;
        }
    }
}