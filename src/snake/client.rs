use bevy::app::App;
use bevy::prelude::*;
use iyes_loopless::prelude::IntoConditionalSystem;
use crate::client::resources::ClientPacketManager;

use crate::common::components::{Direction, Position};
use crate::food::components::Food;
use crate::networking::client_packets::SnakeMovement;
use crate::networking::server_packets::{SnakePositions, SnakePositionsPacketBuilder, SpawnTail, SpawnTailPacketBuilder};
use crate::snake::components::{SnakeHead, SnakeState};
use crate::snake::spawn_tail;
use crate::state::GameState;

pub struct SnakeClientPlugin;

impl Plugin for SnakeClientPlugin {
    
    fn build(&self, app: &mut App) {
        app
            .add_system(update_snake_positions.run_in_state(GameState::Running).label(SnakeState::Movement))
            .add_system(snake_movement_input.run_in_state(GameState::Running).after(SnakeState::Movement));
            //.add_system(handle_spawn_tail.run_in_state(GameState::Running).before(SnakeState::Movement));
    }
}

fn update_snake_positions(mut commands: Commands, mut manager: ResMut<ClientPacketManager>, mut q: Query<(&mut Position, &mut SnakeHead)>, mut tail_positions: Query<&mut Position, (Without<SnakeHead>, Without<Food>)>) {
    let snake_positions = manager.manager.received::<SnakePositions, SnakePositionsPacketBuilder>(false).unwrap();
    if let Some(snake_positions) = snake_positions {
        for snake_position in snake_positions.iter() {
            for orientation in snake_position.positions.iter() {
                for (mut pos, mut head) in q.iter_mut() {
                    pos.x = orientation.position.0;
                    pos.y = orientation.position.1;
                    head.input_direction = orientation.input_direction;
                    head.direction = orientation.direction;
                    
                    let client_tail_len = head.tail.len();
                    let server_tail_len = orientation.tail_positions.len();
                    // TODO: Error handle by despawning and respawning tails on client to match server
                    if client_tail_len > server_tail_len {
                        panic!("Client spawned more tails than server has record of.  This should not happen!");
                    }
                    
                    // Only modify the old tail positions, new ones should already be in the right place
                    for (i, entity) in head.tail.iter().enumerate() {
                        let mut tail_pos = tail_positions.get_mut(*entity).unwrap();
                        tail_pos.x = orientation.tail_positions[i].0;
                        tail_pos.y = orientation.tail_positions[i].1;
                    }

                    // Tail was spawned on server side, spawn on client as well
                    // TODO: instead we can just spawn tails manually above to avoid lag, and remove SpawnTail packet,
                    if client_tail_len < server_tail_len {
                        // This is a blocking call
                        head.tail.append(&mut handle_spawn_tail(&mut commands, &mut manager));
                    }
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

fn handle_spawn_tail(mut commands: &mut Commands, mut manager: &mut ClientPacketManager) -> Vec<Entity> {
    let spawn_tails = manager.manager.received::<SpawnTail, SpawnTailPacketBuilder>(true).unwrap();
    let mut tail_entities = vec![];
    if let Some(spawn_tails) = spawn_tails {
        for st in spawn_tails.iter() {
            tail_entities.push(spawn_tail(&mut commands, Position { x: st.position.0, y: st.position.1 }, None));
        }
    }
    tail_entities
}