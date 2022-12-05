use std::time::Duration;

use bevy::prelude::*;
use bevy::utils::HashMap;
use iyes_loopless::prelude::*;
use rand::random;

use crate::common::components::Position;
use crate::common::constants::{ARENA_HEIGHT, ARENA_WIDTH};
use crate::food::components::Food;
use crate::food::resources::FoodId;
use crate::food::spawn_food;
use crate::networking::server_packets::EatFood;
use crate::server::server::ServerPacketManager;
use crate::snake::components::{SnakeHead, SnakeState};
use crate::snake::spawn_tail;
use crate::state::GameState;

pub struct FoodServerPlugin;

impl Plugin for FoodServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(eat_food.run_in_state(GameState::Running).after(SnakeState::Movement))
            .add_fixed_timestep(Duration::from_secs(1), "spawn_food")
            .add_fixed_timestep_system("spawn_food", 0, auto_spawn_food.run_in_state(GameState::Running));
    }
}

// Server only
fn auto_spawn_food(mut commands: Commands, mut food_id: ResMut<FoodId>, mut manager: ResMut<ServerPacketManager>) {
    let x = (random::<f32>() * ARENA_WIDTH as f32) as i32;
    let y = (random::<f32>() * ARENA_HEIGHT as f32) as i32;
    spawn_food(&mut commands, food_id.as_mut(), Some(&mut manager.manager), x, y);
    food_id.id += 1;
}

fn eat_food(
    mut commands: Commands,
    mut manager: ResMut<ServerPacketManager>,
    foods: Query<(Entity, &Position), With<Food>>,
    mut snakes: Query<(&Position, &mut SnakeHead)>,
    positions: Query<&Position, (Without<SnakeHead>, Without<Food>)>,
) {
    let food_positions = get_food_positions(foods);

    for (position, mut head) in snakes.iter_mut() {
        if let Some(entity) = food_positions.get(position) {
            commands.entity(*entity).despawn();
            manager.manager.send(EatFood { position: (position.x, position.y) }).unwrap();
            let mut position = position;
            if !head.tail.is_empty() {
                position = positions.get(*head.tail.last().unwrap()).unwrap();
            }
            head.tail.push(spawn_tail(&mut commands, *position));
        }
    }
}

#[inline]
fn get_food_positions(foods: Query<(Entity, &Position), With<Food>>) -> HashMap<Position, Entity> {
    let mut food_positions = HashMap::new();
    // Assumes no position has multiple food
    for (entity, position) in foods.iter() {
        food_positions.insert(*position, entity);
    }
    food_positions
}