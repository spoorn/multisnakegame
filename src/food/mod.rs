use bevy::prelude::*;

use networking::packet::PacketManager;

use crate::common::components::Position;
use crate::common::components::Size;
use crate::food::components::Food;
use crate::food::resources::FoodId;
use crate::networking::server_packets::SpawnFood;

pub mod components;
pub mod resources;
pub mod server;
pub mod client;

pub struct FoodPlugin;

impl Plugin for FoodPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup_food);
    }
}

const FOOD_COLOR: Color = Color::rgb(1.0, 0.0, 1.0);

fn setup_food(mut commands: Commands) {
    commands.insert_resource::<FoodId>(FoodId { id: 0 });
}

pub fn spawn_food(commands: &mut Commands, food_id: &mut FoodId, manager: Option<&mut PacketManager>, x: i32, y: i32) {
    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite {
                color: FOOD_COLOR,
                ..default()
            },
            ..default()
        })
        .insert(Food { id: food_id.id })
        .insert(Position { x, y })
        .insert(Size::square(0.8));
    food_id.id += 1;
    if let Some(manager) = manager {
        manager.send(SpawnFood { position: (x, y) }).unwrap();
    }
}
