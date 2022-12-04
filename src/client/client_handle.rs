use bevy::app::AppExit;
use bevy::prelude::*;
use bevy::window::{WindowClosed, WindowCloseRequested};
use iyes_loopless::prelude::{ConditionSet, IntoConditionalSystem};

use crate::client::client::ClientPacketManager;
use crate::common::components::Position;
use crate::food::components::Food;
use crate::food::resources::FoodId;
use crate::food::spawn_food;
use crate::networking::client_packets::Disconnect;
use crate::networking::server_packets::{SnakePositions, SnakePositionsPacketBuilder, SpawnFood, SpawnFoodPacketBuilder};
use crate::snake::components::SnakeHead;
use crate::state::GameState;

pub struct ClientHandlePlugin;

impl Plugin for ClientHandlePlugin {

    fn build(&self, app: &mut App) {
        app
            .add_startup_system(setup_client)
            .add_system_set_to_stage(CoreStage::PreUpdate, 
                                     ConditionSet::new()
                                         .run_in_state(GameState::Running)
                                         .with_system(client_handle_packets)
                                         .into()
                                     )
            .add_system(exit_system.run_not_in_state(GameState::MainMenu));
    }
}

fn setup_client(mut commands: Commands) {
    // is_client
    commands.insert_resource::<bool>(true);
}

fn client_handle_packets(mut manager: ResMut<ClientPacketManager>,
                         mut commands: Commands,
                         mut food_id: ResMut<FoodId>) {
    let manager = &mut manager.manager;

    let snake_positions = manager.received::<SnakePositions, SnakePositionsPacketBuilder>(false).unwrap();
    let mut spawn_foods = manager.received::<SpawnFood, SpawnFoodPacketBuilder>(false).unwrap();
    
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

fn exit_system(mut manager: ResMut<ClientPacketManager>, mut exit: EventReader<AppExit>, mut close_window: EventReader<WindowCloseRequested>) {
    if !exit.is_empty() || !close_window.is_empty() {
        manager.manager.send(Disconnect).unwrap();
    }
}