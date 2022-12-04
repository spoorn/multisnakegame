use std::env;
use bevy::prelude::*;

mod common;
mod food;
mod snake;
mod state;
mod ui;

// Test
mod client;
mod server;
mod networking;

fn main() {
    let args: Vec<String> = env::args().collect();
    let client_or_server = &args[1];
    let server_addr = args[2].to_owned();
    let client_addr = if args.len() >= 4 { args[3].to_owned() } else { "None".to_string() };
    
    if client_or_server == "client" {
        // Client test
        App::new()
            .insert_resource(WindowDescriptor {
                title: "Snake!".to_string(),
                width: 1000.0,
                height: 1000.0,
                // TODO: always opens on primary monitor, can't find the Current monitor for some reason
                position: WindowPosition::Centered(MonitorSelection::Primary),
                ..default()
            })
            .insert_resource(ClearColor(Color::rgb(0.04, 0.04, 0.04)))
            .add_plugins(DefaultPlugins)
            .add_plugin(ui::UiPlugin)
            .add_plugin(common::CommonPlugin)
            .add_plugin(food::FoodPlugin { is_client: true })
            .add_plugin(snake::SnakePlugin)
            .add_plugin(client::client::ClientPlugin { client_addr, server_addr })
            .add_plugin(client::client_handle::ClientHandlePlugin)
            .run();
    } else {
        // Server test
        App::new()
            .add_plugins(DefaultPlugins)
            .add_plugin(server::server::ServerPlugin { server_addr })
            .add_plugin(server::server_handle::ServerHandlePlugin)
            .add_plugin(common::CommonPlugin)
            .add_plugin(food::FoodPlugin { is_client: false })
            .add_plugin(snake::SnakePlugin)
            .run();
    }
}
