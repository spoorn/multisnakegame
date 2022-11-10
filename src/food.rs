use std::time::Duration;

use bevy::prelude::*;
use bevy::utils::HashMap;
use iyes_loopless::prelude::*;
use rand::random;

use crate::common::components::Position;
use crate::common::components::Size;
use crate::common::constants::{ARENA_HEIGHT, ARENA_WIDTH};
use crate::food::components::Food;
use crate::snake::components::{SnakeHead, SnakeState};
use crate::snake::spawn_tail;
use crate::state::GameState;

pub mod components;

pub struct FoodPlugin;

impl Plugin for FoodPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(eat_food.run_in_state(GameState::Running).after(SnakeState::Movement))
            .add_fixed_timestep(Duration::from_secs(1), "spawn_food")
            .add_fixed_timestep_system("spawn_food", 0, spawn_food.run_in_state(GameState::Running));
    }
}

const FOOD_COLOR: Color = Color::rgb(1.0, 0.0, 1.0);

fn spawn_food(mut commands: Commands) {
    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite {
                color: FOOD_COLOR,
                ..default()
            },
            ..default()
        })
        .insert(Food)
        .insert(Position {
            x: (random::<f32>() * ARENA_WIDTH as f32) as i32,
            y: (random::<f32>() * ARENA_HEIGHT as f32) as i32,
        })
        .insert(Size::square(0.8));
}

fn eat_food(
    mut commands: Commands,
    foods: Query<(Entity, &Position), With<Food>>,
    mut snakes: Query<(&Position, &mut SnakeHead)>,
    positions: Query<&Position, (Without<SnakeHead>, Without<Food>)>,
) {
    let food_positions = get_food_positions(foods);

    for (position, mut head) in snakes.iter_mut() {
        if let Some(entity) = food_positions.get(position) {
            commands.entity(*entity).despawn();
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
