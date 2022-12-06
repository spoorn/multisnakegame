use bevy::app::App;
use bevy::prelude::*;
use bevy::utils::HashMap;
use iyes_loopless::prelude::{IntoConditionalSystem, NextState};
use rand::random;

use crate::common::components::Position;
use crate::common::constants::{ARENA_HEIGHT, ARENA_WIDTH};
use crate::common::correct_position_at_ends;
use crate::networking::client_packets::{SnakeMovement, SnakeMovementPacketBuilder, StartNewGame, StartNewGamePacketBuilder};
use crate::networking::server_packets::{SnakePosition, SnakePositions, SpawnSnake, StartNewGameAck};
use crate::server::resources::{ServerInfo, ServerPacketManager};
use crate::snake::components::{SnakeHead, SnakeState};
use crate::snake::{move_snake, spawn_snake};
use crate::snake::resources::SnakeId;
use crate::state::GameState;

pub struct SnakeServerPlugin;

impl Plugin for SnakeServerPlugin {

    fn build(&self, app: &mut App) {
        app.insert_resource(SnakeId { id: 0 })
            .add_system(wait_for_start_game_ack.run_in_state(GameState::MainMenu))
            .add_system(snake_movement.run_in_state(GameState::Running).label(SnakeState::Movement))
            .add_system(update_snake_movement.run_in_state(GameState::Running).before(SnakeState::Movement));
    }
}

fn wait_for_start_game_ack(mut commands: Commands, mut manager: ResMut<ServerPacketManager>, server_info: Res<ServerInfo>, mut snake_id: ResMut<SnakeId>, q: Query<(&Position, &SnakeHead)>) {
    let acks = manager.manager.received_all::<StartNewGame, StartNewGamePacketBuilder>(false).unwrap();
    let num_clients = manager.manager.get_num_clients() as u8;
    for (addr, ack) in acks.iter() {
        if ack.is_some() {
            if num_clients > server_info.want_num_clients {
                panic!("[server] Invalid State: server number of clients exceeded number of wanted clients!");
            } else if num_clients == server_info.want_num_clients {
                info!("[server] Transitioning to PreGame");
                commands.insert_resource(NextState(GameState::PreGame));
            }
            let x = (random::<f32>() * ARENA_WIDTH as f32) as i32;
            let y = (random::<f32>() * ARENA_HEIGHT as f32) as i32;
            info!("[server] Spawned snake at {}, {}", x, y);
            spawn_snake(&mut commands, snake_id.id, Position { x, y });
            manager.manager.send_to(addr, StartNewGameAck { num_snakes: server_info.want_num_clients }).unwrap();
            // TODO: This can probably be optimized rather than broadcasting to all clients everytime, but this is simple
            // Previously spawned snakes.  Assumes client handles duplicates
            for (pos, head) in q.iter() {
                manager.manager.broadcast(SpawnSnake { id: head.id, position: (pos.x, pos.y) }).unwrap();
            }
            manager.manager.broadcast(SpawnSnake { id: snake_id.id, position: (x, y) }).unwrap();  // The newly spawned one
            snake_id.id += 1;
        }
    }
}

fn update_snake_movement(mut head_positions: Query<&mut SnakeHead>, mut manager: ResMut<ServerPacketManager>) {
    let snake_movements = manager.manager.received_all::<SnakeMovement, SnakeMovementPacketBuilder>(false).unwrap();
    for (_addr, movements) in snake_movements.iter() {
        if let Some(movements) = movements {
            let mut snakes = HashMap::new();
            for head in head_positions.iter_mut() {
                snakes.insert(head.id, head);
            }

            // TODO: check which snake to move
            for movement in movements.iter() {
                let snake = snakes.get_mut(&movement.id).unwrap();
                let dir = movement.direction;
                if dir != snake.direction.opposite() {
                    snake.input_direction = dir;
                }
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
            let mut tail_positions = vec![];
            for entity in head.tail.iter() {
                let tail_pos = positions.get(*entity).unwrap();
                tail_positions.push((tail_pos.x, tail_pos.y));
            }
            
            snake_positions.push(SnakePosition {
                id: head.id,
                input_direction: head.input_direction,
                direction: head.direction,
                position: (position.x, position.y),
                tail_positions
            });
        }
    }
    
    if !snake_positions.is_empty() {
        manager.manager.broadcast(SnakePositions { positions: snake_positions }).unwrap();
    }
}