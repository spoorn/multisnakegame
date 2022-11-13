use bevy::prelude::*;

mod common;
mod food;
mod snake;
mod state;
mod ui;

// Test
mod client;
mod server;


#[tokio::main]
async fn main() {
    let server = tokio::spawn(async {
        server::server::run().await.unwrap();
    });
    let client = tokio::spawn(async {
        client::client::run().await.unwrap();
    });

    //client::client::run();
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
        .add_plugin(food::FoodPlugin)
        .add_plugin(snake::SnakePlugin)
        .run();
}
