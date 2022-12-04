use networking_macros::bincode_packet;

#[bincode_packet]
pub struct StartNewGame;

#[bincode_packet]
pub struct Disconnect;