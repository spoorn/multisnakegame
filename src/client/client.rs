use bevy::app::AppExit;
use bevy::prelude::*;
use bevy::window::WindowCloseRequested;
use iyes_loopless::prelude::{AppLooplessStateExt, ConditionSet, IntoConditionalSystem};
use iyes_loopless::state::NextState;

use networking::packet::PacketManager;

use crate::client::resources::{ClientInfo, ClientPacketManager};
use crate::food::resources::FoodId;
use crate::food::spawn_food;
use crate::networking::client_packets::{Disconnect, SnakeMovement, StartNewGame};
use crate::networking::server_packets::{SnakePositions, SnakePositionsPacketBuilder, SpawnFood, SpawnFoodPacketBuilder, StartNewGameAck, StartNewGameAckPacketBuilder};
use crate::state::GameState;

pub struct ClientPlugin {
    pub client_addr: String,
    pub server_addr: String
}

impl Plugin for ClientPlugin {
    
    fn build(&self, app: &mut App) {
        app
            .insert_resource(ClientInfo { client_addr: self.client_addr.to_owned(), server_addr: self.server_addr.to_owned() })
            .add_startup_system(setup_client)
            .add_system_set_to_stage(CoreStage::PreUpdate,
                                     ConditionSet::new()
                                         .run_in_state(GameState::Running)
                                         .with_system(client_handle_packets)
                                         .into()
            )
            .add_system(exit_system.run_not_in_state(GameState::MainMenu))
            .add_enter_system(GameState::ConnectToServer, send_start_game_packet);
    }
}

fn send_start_game_packet(mut commands: Commands, client_info: Res<ClientInfo>) {
    let mut manager = PacketManager::new();
    manager.init_connection(false, 3, 3, client_info.server_addr.to_owned(), Some(client_info.client_addr.to_owned())).unwrap();
    manager.register_receive_packet::<StartNewGameAck>(StartNewGameAckPacketBuilder).unwrap();
    manager.register_receive_packet::<SnakePositions>(SnakePositionsPacketBuilder).unwrap();
    manager.register_receive_packet::<SpawnFood>(SpawnFoodPacketBuilder).unwrap();
    manager.register_send_packet::<StartNewGame>().unwrap();
    manager.register_send_packet::<Disconnect>().unwrap();
    manager.register_send_packet::<SnakeMovement>().unwrap();
    
    manager.send(StartNewGame).unwrap();

    // wait for ack
    manager.received::<StartNewGameAck, StartNewGameAckPacketBuilder>(true).unwrap();
    commands.insert_resource(ClientPacketManager { manager });
    commands.insert_resource(NextState(GameState::PreGame));
}

fn setup_client(mut commands: Commands) {
    // is_client
    commands.insert_resource::<bool>(true);
}

fn client_handle_packets(mut manager: ResMut<ClientPacketManager>,
                         mut commands: Commands,
                         mut food_id: ResMut<FoodId>) {
    let manager = &mut manager.manager;

    let spawn_foods = manager.received::<SpawnFood, SpawnFoodPacketBuilder>(false).unwrap();

    match spawn_foods {
        None => {

        },
        Some(sf) => {
            if !sf.is_empty() {
                for food in sf.iter() {
                    spawn_food(&mut commands, food_id.as_mut(), None, food.position.0, food.position.1);
                }
            }
        }
    }
}

fn exit_system(mut manager: ResMut<ClientPacketManager>, exit: EventReader<AppExit>, close_window: EventReader<WindowCloseRequested>) {
    if !exit.is_empty() || !close_window.is_empty() {
        manager.manager.send(Disconnect).unwrap();
    }
}