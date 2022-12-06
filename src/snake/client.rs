use bevy::app::App;
use bevy::prelude::*;
use bevy::utils::HashMap;
use iyes_loopless::prelude::{IntoConditionalSystem, NextState};

use crate::client::resources::ClientPacketManager;
use crate::common::components::{Direction, Position};
use crate::food::components::Food;
use crate::networking::client_packets::{Ready, SnakeMovement};
use crate::networking::server_packets::{SnakePositions, SnakePositionsPacketBuilder, SpawnSnake, SpawnSnakePacketBuilder, SpawnTail, SpawnTailPacketBuilder, StartNewGameAck, StartNewGameAckPacketBuilder};
use crate::snake::{spawn_snake, spawn_tail};
use crate::snake::components::{SnakeHead, SnakeState};
use crate::snake::resources::{NumSnakesToSpawn, SnakeId};
use crate::state::GameState;

pub struct SnakeClientPlugin;

impl Plugin for SnakeClientPlugin {
    
    fn build(&self, app: &mut App) {
        app.insert_resource(SnakeId { id: 0 })
            .add_system(wait_for_ack.run_in_state(GameState::ConnectToServer))
            .add_system(pre_game.run_in_state(GameState::PreGame))
            .add_system(update_snake_positions.run_in_state(GameState::Running).label(SnakeState::Movement))
            .add_system(snake_movement_input.run_in_state(GameState::Running).after(SnakeState::Movement));
            //.add_system(handle_spawn_tail.run_in_state(GameState::Running).before(SnakeState::Movement));
    }
}

fn wait_for_ack(mut commands: Commands, mut manager: ResMut<ClientPacketManager>) {
    let ack = manager.manager.received::<StartNewGameAck, StartNewGameAckPacketBuilder>(false).unwrap();
    if let Some(ack) = ack {
        if !ack.is_empty() {
            info!("[client] Got StartNewGameAck from server, with expected number of snakes={}", ack[0].num_snakes);
            commands.insert_resource(NumSnakesToSpawn { num: ack[0].num_snakes as i32 });
            commands.insert_resource(NextState(GameState::PreGame));
        }
    }
}

fn pre_game(mut commands: Commands, mut manager: ResMut<ClientPacketManager>, mut num_snakes: ResMut<NumSnakesToSpawn>, mut snake_id: ResMut<SnakeId>) {
    let snake_spawns = manager.manager.received::<SpawnSnake, SpawnSnakePacketBuilder>(false).unwrap();
    if let Some(snake_spawns) = snake_spawns {
        for spawn in snake_spawns.iter() {
            if spawn.id < snake_id.id {
                continue;  // Skip as we already processed this snake spawn
            } else if spawn.id > snake_id.id {
                panic!("[client] Received snake id={} from server that did not match client's tracked id={}", spawn.id, snake_id.id);
            }
            spawn_snake(&mut commands, spawn.id, Position { x: spawn.position.0, y: spawn.position.1 });
            snake_id.id += 1;
            num_snakes.num -= 1;
            if num_snakes.num < 0 {
                panic!("[client] Spawned more snakes than expected!")
            }
        }
        
        if num_snakes.num == 0 {
            manager.send(Ready).unwrap();
        }
    }
}

fn update_snake_positions(mut commands: Commands, mut manager: ResMut<ClientPacketManager>, mut q: Query<(&mut Position, &mut SnakeHead)>, mut tail_positions: Query<&mut Position, (Without<SnakeHead>, Without<Food>)>) {
    let snake_positions = manager.manager.received::<SnakePositions, SnakePositionsPacketBuilder>(false).unwrap();
    if let Some(snake_positions) = snake_positions {
        let mut snakes = HashMap::new();
        for (pos, head) in q.iter_mut() {
            snakes.insert(head.id, (pos, head));
        }
        
        for snake_position in snake_positions.iter() {
            for orientation in snake_position.positions.iter() {
                match snakes.get_mut(&orientation.id) {
                    None => {
                        panic!("[client] Snake with ID {} does not exist!", orientation.id);
                    }
                    Some((pos, head)) => {
                        pos.x = orientation.position.0;
                        pos.y = orientation.position.1;
                        head.input_direction = orientation.input_direction;
                        head.direction = orientation.direction;

                        let client_tail_len = head.tail.len();
                        let server_tail_len = orientation.tail_positions.len();
                        // The client can have 1 more tail if it receives the SpawnTail packet before the server has finished
                        // the tick, but it should never be 2 more tails than the server
                        if client_tail_len > server_tail_len - 1 {
                            panic!("Client spawned more tails {} than server has record of {}.  This should not happen!  \
                            Most likely means there is desync between client and server, or the server spawned multiple tails in one tick.", client_tail_len, server_tail_len);
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
            manager.manager.send(SnakeMovement { id: head.id, direction: head.input_direction }).unwrap();
        }
    }
}

fn handle_spawn_tail(mut commands: &mut Commands, mut manager: &mut ClientPacketManager) -> Vec<Entity> {
    let spawn_tails = manager.manager.received::<SpawnTail, SpawnTailPacketBuilder>(true).unwrap();
    let mut tail_entities = vec![];
    if let Some(spawn_tails) = spawn_tails {
        for st in spawn_tails.iter() {
            tail_entities.push(spawn_tail(&mut commands, Position { x: st.position.0, y: st.position.1 }, None, st.id));
        }
    }
    tail_entities
}