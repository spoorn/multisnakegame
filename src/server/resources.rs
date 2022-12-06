use std::ops::{Deref, DerefMut};
use networking::packet::PacketManager;

pub struct ServerInfo {
    pub server_addr: String,
    pub want_num_clients: u8
}

pub struct ReadyCount {
    pub count: i32
}

pub struct ServerPacketManager {
    pub manager: PacketManager
}

impl Deref for ServerPacketManager {
    type Target = PacketManager;

    fn deref(&self) -> &Self::Target {
        &self.manager
    }
}

impl DerefMut for ServerPacketManager {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.manager
    }
}