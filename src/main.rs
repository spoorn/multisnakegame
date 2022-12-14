// Don't open console window in release mode
//#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{env, thread};

use bevy::diagnostic::DiagnosticsPlugin;
use bevy::input::InputPlugin;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy::winit::{UpdateMode, WinitSettings};
use bevy_embedded_assets::EmbeddedAssetPlugin;

mod common;
mod food;
mod snake;
mod state;
mod ui;

// Test
mod client;
mod server;
mod networking;
mod lobby;

fn hide_console_window() {
    use std::ptr;
    use winapi::um::wincon::GetConsoleWindow;
    use winapi::um::winuser::{ShowWindow, SW_HIDE};

    let window = unsafe {GetConsoleWindow()};
    // https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-showwindow
    if window != ptr::null_mut() {
        unsafe {
            ShowWindow(window, SW_HIDE);
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let client_or_server = if args.len() >= 2 { &args[1] } else { "client" };
    let server_addr = if args.len() >= 3 { args[2].to_owned() } else { "192.168.1.243:28154".to_string() };
    let client_addr = if args.len() >= 4 { args[3].to_owned() } else { "0.0.0.0:5001".to_string() };
    let lobby_server_addr = if args.len() >= 4 { args[3].to_owned() } else { "192.168.1.243:28153".to_string() };
    
    if client_or_server != "client" {
        let server_addr = server_addr.clone();
        thread::spawn(move || {
            let mut first = true;
            loop {
                // Server test
                App::new()
                    .add_plugins_with(MinimalPlugins, |group| {
                        group.add(InputPlugin::default());
                        group.add(TransformPlugin::default());
                        group.add(HierarchyPlugin::default());
                        group.add(DiagnosticsPlugin::default());

                        // LogPlugin should only be globally initialized for all Apps: https://docs.rs/bevy_log/0.8.1/bevy_log/struct.LogPlugin.html
                        if first { group.add(LogPlugin::default()); }
                        group
                    })
                    .insert_resource(WinitSettings {
                        return_from_run: true,
                        focused_mode: UpdateMode::Continuous,
                        unfocused_mode: UpdateMode::Continuous
                    })
                    .add_plugin(server::server::ServerPlugin { server_addr: server_addr.clone() })
                    .add_plugin(common::CommonPlugin { is_client: false })
                    .add_plugin(food::FoodPlugin)
                    .add_plugin(food::server::FoodServerPlugin)
                    .add_plugin(snake::SnakePlugin)
                    .add_plugin(snake::server::SnakeServerPlugin)
                    .run();
                first = false;
            }
        });
    }
    
    if client_or_server == "client" || client_or_server == "both" {
        hide_console_window();
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
            .add_plugins_with(DefaultPlugins, |group| {
                group.add_before::<bevy::asset::AssetPlugin, _>(EmbeddedAssetPlugin);
                if client_or_server == "both" {
                    group.disable::<LogPlugin>();
                }
                return group
            })
            .insert_resource(WinitSettings {
                return_from_run: true,
                focused_mode: UpdateMode::Continuous,
                unfocused_mode: UpdateMode::Continuous
            })
            .add_plugin(ui::UiPlugin)
            .add_plugin(common::CommonPlugin { is_client: true })
            .add_plugin(food::FoodPlugin)
            .add_plugin(food::client::FoodClientPlugin)
            .add_plugin(snake::SnakePlugin)
            .add_plugin(snake::client::SnakeClientPlugin)
            .add_plugin(client::client::ClientPlugin { client_addr, lobby_server_addr: lobby_server_addr.clone(), server_addr: server_addr.clone() })
            .run();
    }
}
