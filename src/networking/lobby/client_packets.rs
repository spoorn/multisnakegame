use durian::bincode_packet;

// Note: We just use the lobby name as the key index.  For optimization, we can switch to integer later

#[bincode_packet]
pub struct CreateLobby {
    pub name: String,
    pub description: String,
    pub max_players: u8,
    pub player_name: String
}

#[bincode_packet]
pub struct CancelLobby {
    pub name: String
}

#[bincode_packet]
pub struct JoinLobby {
    pub name: String,
    pub player_name: String
}

#[bincode_packet]
pub struct LeaveLobby {
    pub name: String
}