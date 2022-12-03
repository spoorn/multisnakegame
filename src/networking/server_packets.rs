use networking_macros::bincode_packet;

#[bincode_packet]
#[derive(Debug)]
pub struct PositionPacket {
    pub id: u32
}

#[bincode_packet]
#[derive(Debug)]
pub struct FoodPacket {
    pub name: String,
    pub item: String,
    pub id: u32
}