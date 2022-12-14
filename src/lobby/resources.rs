use bevy::utils::HashMap;
use durian::{DerefPacketManager, PacketManager};

pub struct LobbyServerInfo {
    pub addr: String
}

#[derive(DerefPacketManager)]
pub struct LobbyPacketManager {
    pub manager: PacketManager
}

pub struct Lobbies {
    pub lobbies: HashMap<String, Lobby>
}

pub struct Lobby {
    pub name: String,
    pub description: String,
    pub leader_addr: String,
    // (addr, player name)
    pub players: Vec<(String, String)>,
    pub max_players: u8
}