use bevy::prelude::*;
use bevy::utils::HashMap;

use durian::PacketManager;

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

pub fn spawn_food(commands: &mut Commands, food_id: &mut FoodId, manager: Option<&mut PacketManager>, position: Position) {
    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite {
                color: FOOD_COLOR,
                ..default()
            },
            ..default()
        })
        .insert(Food { id: food_id.id })
        .insert(position)
        .insert(Size::square(0.8));
    food_id.id += 1;
    if let Some(manager) = manager {
        manager.broadcast(SpawnFood { position: (position.x, position.y) }).unwrap();
    }
}

#[inline]
fn get_food_positions(foods: Query<(Entity, &Position), With<Food>>) -> HashMap<Position, Entity> {
    let mut food_positions: HashMap<Position, Entity> = HashMap::new();
    // Assumes there is only one food per position
    for (entity, position) in foods.iter() {
        food_positions.insert(*position, entity);
    }
    food_positions
}
