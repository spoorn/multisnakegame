use bevy::prelude::*;
use rand::Rng;
use tokio::runtime::Runtime;

use networking::packet::PacketManager;

use crate::networking::client_packets::{OtherPacket, OtherPacketPacketBuilder, TestPacket, TestPacketPacketBuilder};
use crate::networking::server_packets::{FoodPacket, PositionPacket};

pub struct ServerPlugin;

impl Plugin for ServerPlugin {

    fn build(&self, app: &mut App) {
        app.add_startup_system(setup_packet_manager)
            .add_system(test_server_packets);
    }
}

struct ServerPacketManager {
    manager: PacketManager
}

fn test_server_packets(mut manager: ResMut<ServerPacketManager>, runtime: Res<ServerTokioRuntime>) {
    runtime.runtime.block_on(async {
        let mut manager = &mut manager.manager;
        let mut rng = rand::thread_rng();
        if rng.gen_range(0..5) == 0 {
            manager.send(PositionPacket { id: 2 }).await.unwrap();
            manager.send(FoodPacket { name: "spoorn".to_string(), item: "kiko".to_string(), id: 2 }).await.unwrap();

            let pos_packets = manager.received::<TestPacket, TestPacketPacketBuilder>(false).await;
            println!("[server] got packet {:?}", pos_packets);
            let food_packets = manager.received::<OtherPacket, OtherPacketPacketBuilder>(false).await;
            println!("[server] got packet {:?}", food_packets);
        }
    });
}

struct ServerTokioRuntime {
    runtime: Runtime
}

fn setup_packet_manager(mut commands: Commands) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let mut commands = runtime.block_on(async move {
        let mut manager = PacketManager::new();
        manager.init_connection(true, 2, 2, "127.0.0.1:5000", None).await.unwrap();
        manager.register_receive_packet::<TestPacket>(TestPacketPacketBuilder).unwrap();
        manager.register_receive_packet::<OtherPacket>(OtherPacketPacketBuilder).unwrap();
        manager.register_send_packet::<PositionPacket>().unwrap();
        manager.register_send_packet::<FoodPacket>().unwrap();
        commands.insert_resource(ServerPacketManager { manager });
        commands
    });
    commands.insert_resource(ServerTokioRuntime { runtime });
}