use networking::packet::PacketManager;

pub struct ClientInfo {
    pub client_addr: String,
    pub server_addr: String
}

pub struct ClientPacketManager {
    pub manager: PacketManager
}