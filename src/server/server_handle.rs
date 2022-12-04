use bevy::app::AppExit;
use bevy::prelude::*;
use iyes_loopless::prelude::{AppLooplessStateExt, ConditionSet, IntoConditionalSystem};
use iyes_loopless::state::NextState;

use crate::common::components::Position;
use crate::food::components::Food;
use crate::networking::client_packets::{Disconnect, DisconnectPacketBuilder, StartNewGame, StartNewGamePacketBuilder};
use crate::networking::server_packets::StartNewGameAck;
use crate::server::server::ServerPacketManager;
use crate::snake::components::SnakeHead;
use crate::state::GameState;

pub struct ServerHandlePlugin;

impl Plugin for ServerHandlePlugin {

    fn build(&self, app: &mut App) {
        app.add_startup_system(setup_server)
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

fn setup_server(mut commands: Commands) {
    // is_client
    commands.insert_resource::<bool>(false);
}

fn wait_for_start_game_ack(mut commands: Commands, mut manager: ResMut<ServerPacketManager>) {
    let ack = manager.manager.received::<StartNewGame, StartNewGamePacketBuilder>(false).unwrap();
    if ack.is_some() {
        commands.insert_resource(NextState(GameState::Running));
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