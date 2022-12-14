use durian::bincode_packet;

// Note: We just use the lobby name as the key index.  For optimization, we can switch to integer later

#[bincode_packet]
pub struct CreateLobbySuccess {
    pub name: String
}

#[bincode_packet]
pub struct CreateLobbyFailed {
    pub name: String,
    pub reason: String
}

#[bincode_packet]
pub struct CancelLobbySuccess {
    pub name: String
}

#[bincode_packet]
pub struct CancelLobbyFailed {
    pub name: String,
    pub reason: String
}

#[bincode_packet]
pub struct JoinLobbySuccess {
    pub name: String
}

#[bincode_packet]
pub struct JoinLobbyFailed {
    pub name: String,
    pub reason: String
}

#[bincode_packet]
pub struct PlayerJoined {
    pub player_name: String,
    pub lobby_name: String
}

#[bincode_packet]
pub struct PlayerLeft {
    pub player_name: String,
    pub lobby_name: String
}

#[bincode_packet]
pub struct LobbyCanceled {
    pub name: String,
    pub reason: String
}

#[bincode_packet]
pub struct LobbyFull {
    pub name: String
}