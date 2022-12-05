use std::time::Duration;

use bevy::prelude::*;
use iyes_loopless::prelude::*;
use rand::random;

use crate::common::components::Position;
use crate::common::constants::{ARENA_HEIGHT, ARENA_WIDTH};
use crate::food::{get_food_positions, spawn_food};
use crate::food::components::Food;
use crate::food::resources::FoodId;
use crate::networking::server_packets::EatFood;
use crate::server::server::ServerPacketManager;
use crate::snake::components::{SnakeHead, SnakeState};
use crate::snake::spawn_tail;
use crate::state::GameState;

pub struct FoodServerPlugin;

impl Plugin for FoodServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(eat_food.run_in_state(GameState::Running).label(SnakeState::EatFood).after(SnakeState::Movement))
            .add_fixed_timestep(Duration::from_secs(1), "spawn_food")
            .add_fixed_timestep_system("spawn_food", 0, auto_spawn_food.run_in_state(GameState::Running));
    }
}

// Server only
fn auto_spawn_food(mut commands: Commands, mut food_id: ResMut<FoodId>, mut manager: ResMut<ServerPacketManager>, foods: Query<(Entity, &Position), With<Food>>) {
    // TODO: calculate only once per frame
    let food_positions = get_food_positions(foods);
    // We don't allow spawning multiple food in the same position, to simplify the snake tail extension logic because
    // Components are only updated once per frame, and messy to figure out which position to place the extraneous tails
    let mut position: Option<Position> = None;
    for _ in 0..5 {
        let x = (random::<f32>() * ARENA_WIDTH as f32) as i32;
        let y = (random::<f32>() * ARENA_HEIGHT as f32) as i32;
        let random_pos = Position { x, y };
        if !food_positions.contains_key(&random_pos) {
            position = Some(random_pos);
            break;
        }
    }
    
    match position {
        None => {
            // Should be extremely rare to happen
            warn!("[server] Could not find an open position to spawn food after 5 tries.  Skipping for this frame.")
        }
        Some(position) => {
            spawn_food(&mut commands, food_id.as_mut(), Some(&mut manager.manager), position);
            info!("[server] Spawned food at {:?}", position);
            food_id.id += 1;
        }
    }
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
            info!("[server] Ate food at {:?}", position);
            // TODO: Send EatFood all in one go per frame
            manager.manager.send(EatFood { position: (position.x, position.y) }).unwrap();
            let mut position = position;
            if !head.tail.is_empty() {
                position = positions.get(*head.tail.last().unwrap()).unwrap();
            }
            head.tail.push(spawn_tail(&mut commands, *position, Some(manager.as_mut())));
        }
    }
}