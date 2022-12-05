use bevy::app::App;
use bevy::prelude::*;
use bevy::utils::HashMap;
use iyes_loopless::prelude::IntoConditionalSystem;
use crate::common::components::Position;
use crate::common::correct_position_at_ends;
use crate::networking::client_packets::{SnakeMovement, SnakeMovementPacketBuilder};
use crate::networking::server_packets::{SnakePosition, SnakePositions};
use crate::server::server::ServerPacketManager;
use crate::snake::components::{SnakeHead, SnakeState};
use crate::snake::move_snake;
use crate::state::GameState;

pub struct SnakeServerPlugin;

impl Plugin for SnakeServerPlugin {

    fn build(&self, app: &mut App) {
        app.add_system(snake_movement.run_in_state(GameState::Running).label(SnakeState::Movement))
            .add_system(update_snake_movement.run_in_state(GameState::Running).before(SnakeState::Movement));
    }
}

fn update_snake_movement(mut head_positions: Query<&mut SnakeHead>, mut manager: ResMut<ServerPacketManager>) {
    let movements = manager.manager.received::<SnakeMovement, SnakeMovementPacketBuilder>(false).unwrap();
    if let Some(movements) = movements {
        let mut snakes = HashMap::new();
        for head in head_positions.iter_mut() {
            snakes.insert(0, head);
        }
        
        // TODO: check which snake to move
        for movement in movements.iter() {
            let snake = snakes.get_mut(&0).unwrap();
            let dir = movement.direction;
            if dir != snake.direction.opposite() {
                snake.input_direction = dir;
            }
        }
    }
}

fn snake_movement(
    time: Res<Time>,
    mut manager: ResMut<ServerPacketManager>,
    mut head_positions: Query<(&mut Position, &mut SnakeHead)>,
    mut positions: Query<&mut Position, Without<SnakeHead>>,
) {
    let mut snake_positions = vec![];
    for (mut position, mut head) in head_positions.iter_mut() {
        let moved = move_snake(time.delta(), position.as_mut(), head.as_mut(), &mut positions);
        correct_position_at_ends(position.as_mut());
        if moved {
            snake_positions.push(SnakePosition {
                input_direction: head.input_direction,
                direction: head.direction,
                position: (position.x, position.y)
            });
        }
    }
    
    if !snake_positions.is_empty() {
        manager.manager.send(SnakePositions { positions: snake_positions }).unwrap();
    }
}