use serde::{Deserialize, Serialize};

use networking_macros::bincode_packet;

use crate::common::components::Direction;

#[bincode_packet]
pub struct StartNewGameAck {
    pub num_snakes: u8
}

#[bincode_packet]
pub struct ReadyAck;

#[bincode_packet]
pub struct SpawnSnake {
    pub id: u8,
    pub position: (i32, i32),
    pub sRGB: (f32, f32, f32)
}

#[bincode_packet]
pub struct SpawnFood {
    pub position: (i32, i32)
}

#[bincode_packet]
pub struct SnakePositions {
    pub positions: Vec<SnakePosition>
}

#[derive(Serialize, Deserialize)]
pub struct SnakePosition {
    pub id: u8,
    pub input_direction: Direction,
    pub direction: Direction,
    pub position: (i32, i32),
    pub tail_positions: Vec<(i32, i32)>
}

#[bincode_packet]
pub struct EatFood {
    pub id: u8,
    pub position: (i32, i32)
}

#[bincode_packet]
pub struct SpawnTail {
    pub id: u8,
    pub position: (i32, i32)
}