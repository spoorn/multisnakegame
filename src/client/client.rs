use bevy::prelude::*;
use iyes_loopless::prelude::AppLooplessStateExt;
use iyes_loopless::state::NextState;

use networking::packet::PacketManager;

use crate::networking::client_packets::StartNewGame;
use crate::networking::server_packets::{SnakePositions, SnakePositionsPacketBuilder, SpawnFood, SpawnFoodPacketBuilder, StartNewGameAck, StartNewGameAckPacketBuilder};
use crate::state::GameState;

pub struct ClientPlugin {
    pub client_addr: String,
    pub server_addr: String
}

impl Plugin for ClientPlugin {
    
    fn build(&self, app: &mut App) {
        app
            .add_enter_system(GameState::ConnectToServer, send_start_game_packet);
    }
}

pub struct ClientPacketManager {
    pub manager: PacketManager
}

fn send_start_game_packet(mut commands: Commands) {
    let mut manager = PacketManager::new();
    manager.init_connection(false, 3, 1, "127.0.0.1:5000", Some("127.0.0.1:5001")).unwrap();
    manager.register_receive_packet::<StartNewGameAck>(StartNewGameAckPacketBuilder).unwrap();
    manager.register_receive_packet::<SnakePositions>(SnakePositionsPacketBuilder).unwrap();
    manager.register_receive_packet::<SpawnFood>(SpawnFoodPacketBuilder).unwrap();
    manager.register_send_packet::<StartNewGame>().unwrap();
    
    manager.send(StartNewGame).unwrap();

    // wait for ack
    manager.received::<StartNewGameAck, StartNewGameAckPacketBuilder>(true).unwrap();
    commands.insert_resource(ClientPacketManager { manager });
    commands.insert_resource(NextState(GameState::PreGame));
}