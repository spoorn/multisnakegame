use bevy::prelude::*;
use rand::Rng;

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

fn test_server_packets(mut manager: ResMut<ServerPacketManager>) {
    let manager = &mut manager.manager;
    let mut rng = rand::thread_rng();
    if rng.gen_range(0..5) == 0 {
        manager.send(PositionPacket { id: 2 }).unwrap();
        manager.send(FoodPacket { name: "spoorn".to_string(), item: "kiko".to_string(), id: 2 }).unwrap();

        let pos_packets = manager.received::<TestPacket, TestPacketPacketBuilder>(false);
        println!("[server] got packet {:?}", pos_packets);
        let food_packets = manager.received::<OtherPacket, OtherPacketPacketBuilder>(false);
        println!("[server] got packet {:?}", food_packets);
    }
}

fn setup_packet_manager(mut commands: Commands) {
    let mut manager = PacketManager::new();
    manager.init_connection(true, 2, 2, "127.0.0.1:5000", None).unwrap();
    manager.register_receive_packet::<TestPacket>(TestPacketPacketBuilder).unwrap();
    manager.register_receive_packet::<OtherPacket>(OtherPacketPacketBuilder).unwrap();
    manager.register_send_packet::<PositionPacket>().unwrap();
    manager.register_send_packet::<FoodPacket>().unwrap();
    commands.insert_resource(ServerPacketManager { manager });
}