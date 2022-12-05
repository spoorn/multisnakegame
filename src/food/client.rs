use bevy::prelude::*;
use bevy::utils::HashMap;
use iyes_loopless::prelude::ConditionSet;

use crate::client::resources::ClientPacketManager;
use crate::common::components::Position;
use crate::food::components::Food;
use crate::food::resources::FoodId;
use crate::food::spawn_food;
use crate::networking::server_packets::{EatFood, EatFoodPacketBuilder, SpawnFood, SpawnFoodPacketBuilder};
use crate::state::GameState;

pub struct FoodClientPlugin;
impl Plugin for FoodClientPlugin {

    fn build(&self, app: &mut App) {
        app
            .add_system_set_to_stage(CoreStage::PreUpdate,
                                     ConditionSet::new()
                                         .run_in_state(GameState::Running)
                                         .with_system(handle_spawn_food)
                                         .with_system(handle_eat_food)
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
            spawn_food(&mut commands, food_id.as_mut(), None, food.position.0, food.position.1);
        }
    }
}

fn handle_eat_food(mut commands: Commands, mut manager: ResMut<ClientPacketManager>, foods: Query<(Entity, &Position), With<Food>>) {
    let eat_foods = manager.manager.received::<EatFood, EatFoodPacketBuilder>(false).unwrap();
    if let Some(eat_foods) = eat_foods {
        let mut pos_to_food = HashMap::new();
        for (entity, pos) in foods.iter() {
            pos_to_food.insert(pos, entity);
        }
        for eat_food in eat_foods.iter() {
            let position = Position { x: eat_food.position.0, y: eat_food.position.1 };
            if let Some(entity) = pos_to_food.get(&position) {
                commands.entity(*entity).despawn();
            } else {
                warn!("Received EatFood packet for position={:?}, but did not find food there", position);
            }
        }
    }
}