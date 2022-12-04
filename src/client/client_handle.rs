use bevy::prelude::*;
use iyes_loopless::prelude::ConditionSet;

use crate::client::client::ClientPacketManager;
use crate::common::components::Position;
use crate::food::components::Food;
use crate::food::resources::FoodId;
use crate::food::spawn_food;
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
                                     );
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
                    spawn_food(&mut commands, food_id.id, None, food.position.0, food.position.1);
                }
            }
        }
    }
}