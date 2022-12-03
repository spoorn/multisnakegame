use bevy::prelude::*;
use futures_lite::future;
use networking::packet::PacketManager;

pub struct ServerPlugin;

impl Plugin for ServerPlugin {

    fn build(&self, app: &mut App) {
        app.add_startup_system(setup_packet_manager);
    }
}

struct ServerPacketManager {
    manager: PacketManager
}

#[tokio::main]
async fn setup_packet_manager(mut commands: Commands) {
    future::block_on(async move {
        let mut manager = PacketManager::new();
        manager.init_connection(true, 2, 2, "127.0.0.1:5000", None).await.unwrap();
        commands.insert_resource(ServerPacketManager { manager });
    });
}