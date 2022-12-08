use std::ops::{Deref, DerefMut};
use durian::PacketManager;

pub struct ClientInfo {
    pub client_addr: String,
    pub server_addr: String
}

pub struct ClientPacketManager {
    pub manager: PacketManager
}

impl Deref for ClientPacketManager {
    type Target = PacketManager;

    fn deref(&self) -> &Self::Target {
        &self.manager
    }
}

impl DerefMut for ClientPacketManager {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.manager
    }
}