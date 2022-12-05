use bevy::app::AppExit;
use bevy::prelude::*;
use iyes_loopless::prelude::{AppLooplessStateExt, ConditionSet, IntoConditionalSystem, NextState};

use networking::packet::PacketManager;

use crate::common::components::Position;
use crate::food::components::Food;
use crate::networking::client_packets::{Disconnect, DisconnectPacketBuilder, SnakeMovement, SnakeMovementPacketBuilder, StartNewGame, StartNewGamePacketBuilder};
use crate::networking::server_packets::{EatFood, SnakePositions, SpawnFood, SpawnTail, StartNewGameAck};
use crate::server::resources::ServerInfo;
use crate::snake::components::SnakeHead;
use crate::state::GameState;

pub struct ServerPlugin {
    pub server_addr: String
}

impl Plugin for ServerPlugin {

    fn build(&self, app: &mut App) {
        app.insert_resource(ServerInfo { server_addr: self.server_addr.to_owned() })
            .add_startup_system(setup_packet_manager)
            .add_loopless_state(GameState::MainMenu)
            .add_system(wait_for_start_game_ack.run_in_state(GameState::MainMenu))
            .add_system_set_to_stage(CoreStage::Last,
                                     ConditionSet::new()
                                         .run_in_state(GameState::Running)
                                         .with_system(server_handle_packets)
                                         .into())
            .add_system(client_disconnect.run_not_in_state(GameState::MainMenu));
    }
}

pub struct ServerPacketManager {
    pub manager: PacketManager
}

fn setup_packet_manager(mut commands: Commands, server_info: Res<ServerInfo>) {
    let mut manager = PacketManager::new();
    manager.init_connection(true, 3, 5, server_info.server_addr.to_owned(), None).unwrap();
    manager.register_receive_packet::<StartNewGame>(StartNewGamePacketBuilder).unwrap();
    manager.register_receive_packet::<Disconnect>(DisconnectPacketBuilder).unwrap();
    manager.register_receive_packet::<SnakeMovement>(SnakeMovementPacketBuilder).unwrap();
    manager.register_send_packet::<StartNewGameAck>().unwrap();
    manager.register_send_packet::<SnakePositions>().unwrap();
    manager.register_send_packet::<SpawnFood>().unwrap();
    manager.register_send_packet::<EatFood>().unwrap();
    manager.register_send_packet::<SpawnTail>().unwrap();
    commands.insert_resource(ServerPacketManager { manager });
}

fn wait_for_start_game_ack(mut commands: Commands, mut manager: ResMut<ServerPacketManager>) {
    let ack = manager.manager.received::<StartNewGame, StartNewGamePacketBuilder>(false).unwrap();
    if ack.is_some() {
        commands.insert_resource(NextState(GameState::PreGame));
        manager.manager.send(StartNewGameAck).unwrap();
    }
}

fn server_handle_packets(mut manager: ResMut<ServerPacketManager>,
                         q: Query<(&Position, Option<&SnakeHead>, Option<&Food>)>) {
    let manager = &mut manager.manager;

    let mut snake_positions = vec![];
    for (pos, head, food) in q.iter() {
        if head.is_some() {
            snake_positions.push((pos.x, pos.y))
        }
    }

    // let snake_pos_packet = SnakePositions { head_positions: snake_positions };
    // manager.send(snake_pos_packet).unwrap();
}

fn client_disconnect(mut manager: ResMut<ServerPacketManager>, mut exit: EventWriter<AppExit>) {
    let disconnects = manager.manager.received::<Disconnect, DisconnectPacketBuilder>(false).unwrap();
    // TODO: Check all clients
    if disconnects.is_some() && disconnects.unwrap().len() > 0 {
        exit.send(AppExit);
    }
}