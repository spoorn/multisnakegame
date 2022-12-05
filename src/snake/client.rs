use bevy::app::App;
use bevy::prelude::*;
use iyes_loopless::prelude::IntoConditionalSystem;
use crate::client::resources::ClientPacketManager;

use crate::common::components::{Direction, Position};
use crate::networking::client_packets::SnakeMovement;
use crate::networking::server_packets::{SnakePositions, SnakePositionsPacketBuilder};
use crate::snake::components::{SnakeHead, SnakeState};
use crate::state::GameState;

pub struct SnakeClientPlugin;

impl Plugin for SnakeClientPlugin {
    
    fn build(&self, app: &mut App) {
        app
            .add_system(update_snake_positions.run_in_state(GameState::Running).label(SnakeState::Movement))
            .add_system(snake_movement_input.run_in_state(GameState::Running).after(SnakeState::Movement));
    }
}

fn update_snake_positions(mut manager: ResMut<ClientPacketManager>, mut q: Query<(&mut Position, &mut SnakeHead)>) {
    let snake_positions = manager.manager.received::<SnakePositions, SnakePositionsPacketBuilder>(false).unwrap();
    if let Some(snake_positions) = snake_positions {
        for snake_position in snake_positions.iter() {
            for orientation in snake_position.positions.iter() {
                for (mut pos, mut head) in q.iter_mut() {
                    pos.x = orientation.position.0;
                    pos.y = orientation.position.1;
                    head.input_direction = orientation.input_direction;
                    head.direction = orientation.direction;
                }
            }
        }
    }
}

fn snake_movement_input(keys: Res<Input<KeyCode>>, mut head_positions: Query<&mut SnakeHead>, mut manager: ResMut<ClientPacketManager>) {
    // TODO: only control self
    for mut head in head_positions.iter_mut() {
        let dir: Direction = if keys.pressed(KeyCode::Left) {
            Direction::Left
        } else if keys.pressed(KeyCode::Down) {
            Direction::Down
        } else if keys.pressed(KeyCode::Up) {
            Direction::Up
        } else if keys.pressed(KeyCode::Right) {
            Direction::Right
        } else {
            head.input_direction
        };
        if dir != head.direction.opposite() && dir != head.input_direction {
            head.input_direction = dir;
            manager.manager.send(SnakeMovement { direction: head.input_direction }).unwrap();
        }
    }
}