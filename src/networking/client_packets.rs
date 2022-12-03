use networking_macros::bincode_packet;

#[bincode_packet]
#[derive(Debug)]
pub struct TestPacket {
    pub id: u32
}

#[bincode_packet]
#[derive(Debug)]
pub struct OtherPacket {
    pub name: String,
    pub id: u32
}