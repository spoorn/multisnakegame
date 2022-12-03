use bevy::prelude::*;
use futures_lite::future;
use networking::packet::PacketManager;

pub struct ClientPlugin;

impl Plugin for ClientPlugin {
    
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup_packet_manager);
    }
}

struct ClientPacketManager {
    manager: PacketManager
}

#[tokio::main]
async fn setup_packet_manager(mut commands: Commands) {
    future::block_on(async move {
        let mut manager = PacketManager::new();
        manager.init_connection(false, 2, 2, "127.0.0.1:5000", Some("127.0.0.1:5001")).await.unwrap();
        commands.insert_resource(ClientPacketManager { manager });
    });
}