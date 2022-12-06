use networking_macros::bincode_packet;
use crate::common::components::Direction;

#[bincode_packet]
pub struct StartNewGame;

#[bincode_packet]
pub struct Disconnect;

#[bincode_packet]
pub struct Ready;

#[bincode_packet]
pub struct SnakeMovement {
    pub id: u8,
    pub direction: Direction
}