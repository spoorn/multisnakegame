use bevy::prelude::*;

use networking::packet::PacketManager;

use crate::networking::client_packets::{Disconnect, DisconnectPacketBuilder, StartNewGame, StartNewGamePacketBuilder};
use crate::networking::server_packets::{SnakePositions, SpawnFood, StartNewGameAck};
use crate::server::resources::ServerInfo;

pub struct ServerPlugin {
    pub server_addr: String
}

impl Plugin for ServerPlugin {

    fn build(&self, app: &mut App) {
        app.insert_resource(ServerInfo { server_addr: self.server_addr.to_owned() })
            .add_startup_system(setup_packet_manager);
    }
}

pub struct ServerPacketManager {
    pub manager: PacketManager
}

fn setup_packet_manager(mut commands: Commands, server_info: Res<ServerInfo>) {
    let mut manager = PacketManager::new();
    manager.init_connection(true, 2, 3, server_info.server_addr.to_owned(), None).unwrap();
    manager.register_receive_packet::<StartNewGame>(StartNewGamePacketBuilder).unwrap();
    manager.register_receive_packet::<Disconnect>(DisconnectPacketBuilder).unwrap();
    manager.register_send_packet::<StartNewGameAck>().unwrap();
    manager.register_send_packet::<SnakePositions>().unwrap();
    manager.register_send_packet::<SpawnFood>().unwrap();
    commands.insert_resource(ServerPacketManager { manager });
}