use bevy::app::App;
use bevy::prelude::*;
use bevy::utils::HashMap;
use crate::networking::client_packets::{SnakeMovement, SnakeMovementPacketBuilder};
use crate::server::server::ServerPacketManager;
use crate::snake::components::SnakeHead;

pub struct SnakeServerPlugin;

impl Plugin for SnakeServerPlugin {

    fn build(&self, app: &mut App) {
        app.add_system(update_snake_movement);
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
                snake.direction = dir;
            }
        }
    }
}