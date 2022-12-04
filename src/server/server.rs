use bevy::prelude::*;

use networking::packet::PacketManager;

use crate::networking::client_packets::{StartNewGame, StartNewGamePacketBuilder};
use crate::networking::server_packets::{SnakePositions, SpawnFood, StartNewGameAck};

pub struct ServerPlugin {
    pub server_addr: String
}

impl Plugin for ServerPlugin {

    fn build(&self, app: &mut App) {
        app.add_startup_system(setup_packet_manager);
    }
}

pub struct ServerPacketManager {
    pub manager: PacketManager
}

fn setup_packet_manager(mut commands: Commands) {
    let mut manager = PacketManager::new();
    manager.init_connection(true, 1, 3, "127.0.0.1:5000", None).unwrap();
    manager.register_receive_packet::<StartNewGame>(StartNewGamePacketBuilder).unwrap();
    manager.register_send_packet::<StartNewGameAck>().unwrap();
    manager.register_send_packet::<SnakePositions>().unwrap();
    manager.register_send_packet::<SpawnFood>().unwrap();
    commands.insert_resource(ServerPacketManager { manager });
}