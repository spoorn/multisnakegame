use networking_macros::bincode_packet;

#[bincode_packet]
pub struct StartNewGameAck;

#[bincode_packet]
pub struct SpawnFood {
    pub position: (i32, i32)
}

#[bincode_packet]
pub struct SnakePositions {
    pub head_positions: Vec<(i32, i32)>
}