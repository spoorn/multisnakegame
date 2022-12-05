use serde::{Deserialize, Serialize};

use networking_macros::bincode_packet;

use crate::common::components::Direction;

#[bincode_packet]
pub struct StartNewGameAck;

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
    pub input_direction: Direction,
    pub direction: Direction,
    pub position: (i32, i32)
}

#[bincode_packet]
pub struct EatFood {
    pub position: (i32, i32)
}