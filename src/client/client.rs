use bevy::prelude::*;

use networking::packet::PacketManager;

use crate::networking::client_packets::{OtherPacket, TestPacket};
use crate::networking::server_packets::{FoodPacket, FoodPacketPacketBuilder, PositionPacket, PositionPacketPacketBuilder};

pub struct ClientPlugin;

impl Plugin for ClientPlugin {
    
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup_packet_manager)
            .add_system(test_client_packets);
    }
}

struct ClientPacketManager {
    manager: PacketManager
}

fn test_client_packets(mut manager: ResMut<ClientPacketManager>) {
    let manager = &mut manager.manager;
    manager.send(TestPacket { id: 2 }).unwrap();
    manager.send(OtherPacket { name: "spoorn".to_string(), id: 2 }).unwrap();

    let pos_packets = manager.received::<PositionPacket, PositionPacketPacketBuilder>(false);
    println!("[client] got packet {:?}", pos_packets);
    let food_packets = manager.received::<FoodPacket, FoodPacketPacketBuilder>(false);
    println!("[client] got packet {:?}", food_packets);
}

fn setup_packet_manager(mut commands: Commands) {
    let mut manager = PacketManager::new();
    manager.init_connection(false, 2, 2, "127.0.0.1:5000", Some("127.0.0.1:5001")).unwrap();
    manager.register_receive_packet::<PositionPacket>(PositionPacketPacketBuilder).unwrap();
    manager.register_receive_packet::<FoodPacket>(FoodPacketPacketBuilder).unwrap();
    manager.register_send_packet::<TestPacket>().unwrap();
    manager.register_send_packet::<OtherPacket>().unwrap();
    commands.insert_resource(ClientPacketManager { manager });
}