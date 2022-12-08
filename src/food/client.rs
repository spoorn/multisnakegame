use bevy::prelude::*;
use iyes_loopless::prelude::ConditionSet;

use crate::client::resources::ClientPacketManager;
use crate::common::components::Position;
use crate::food::{get_food_positions, spawn_food};
use crate::food::components::Food;
use crate::food::resources::{FoodId, ToEatfood};
use crate::networking::server_packets::{EatFood, EatFoodPacketBuilder, SpawnFood, SpawnFoodPacketBuilder};
use crate::snake::components::SnakeState;
use crate::state::GameState;

pub struct FoodClientPlugin;
impl Plugin for FoodClientPlugin {

    fn build(&self, app: &mut App) {
        app.insert_resource(ToEatfood { positions: Vec::new() })
            .add_system_set_to_stage(CoreStage::PreUpdate,
                                     ConditionSet::new()
                                         .run_in_state(GameState::Running)
                                         .label(SnakeState::EatFood)
                                         .with_system(handle_eat_food)
                                         .into()
            ).add_system_set_to_stage(CoreStage::Update,
                                      ConditionSet::new()
                                          .run_in_state(GameState::Running)
                                          .before(SnakeState::EatFood)
                                          .with_system(handle_spawn_food)
                                          .into()
        );
    }
}

fn handle_spawn_food(mut manager: ResMut<ClientPacketManager>,
                     mut commands: Commands,
                     mut food_id: ResMut<FoodId>) {
    let manager = &mut manager.manager;

    let spawn_foods = manager.received::<SpawnFood, SpawnFoodPacketBuilder>(false).unwrap();
    if let Some(spawn_foods) = spawn_foods {
        for food in spawn_foods.iter() {
            let position = Position { x: food.position.0, y: food.position.1 };
            spawn_food(&mut commands, food_id.as_mut(), None, position);
            info!("[client] spawned food at {:?}", position);
        }
    }
}

fn handle_eat_food(mut commands: Commands, mut manager: ResMut<ClientPacketManager>, mut to_eat: ResMut<ToEatfood>, foods: Query<(Entity, &Position), With<Food>>) {
    // TODO: optimize
    let mut pos_to_food;
    
    let eat_foods = manager.manager.received::<EatFood, EatFoodPacketBuilder>(false).unwrap();
    let has_to_eat = !to_eat.positions.is_empty();
    if eat_foods.is_some() || has_to_eat {
        pos_to_food = get_food_positions(foods);

        if has_to_eat {
            to_eat.positions.retain(|pos| {
                if let Some(entity) = pos_to_food.get(pos) {
                    commands.entity(*entity).despawn();
                    info!("[client] Later ate food at {:?}", pos);
                    return true;
                }
                false
            });
        }
        
        if let Some(eat_foods) = eat_foods {
            for eat_food in eat_foods.iter() {
                let position = Position { x: eat_food.position.0, y: eat_food.position.1 };
                if let Some(entity) = pos_to_food.get(&position) {
                    commands.entity(*entity).despawn();
                    info!("[client] Ate food at {:?}", position);
                } else {
                    warn!("Received EatFood packet for position={:?}, but did not find food there.  Queueing for next tick", position);
                    to_eat.positions.push(position);
                }
            }
        }
    }
}