use bevy::prelude::*;
use iyes_loopless::prelude::ConditionSet;

use crate::client::resources::ClientPacketManager;
use crate::food::resources::FoodId;
use crate::food::spawn_food;
use crate::networking::server_packets::{SpawnFood, SpawnFoodPacketBuilder};
use crate::state::GameState;

pub struct FoodClientPlugin;
impl Plugin for FoodClientPlugin {

    fn build(&self, app: &mut App) {
        app
            .add_system_set_to_stage(CoreStage::PreUpdate,
                                     ConditionSet::new()
                                         .run_in_state(GameState::Running)
                                         .with_system(handle_spawn_food)
                                         .into()
            );
    }
}

fn handle_spawn_food(mut manager: ResMut<ClientPacketManager>,
                     mut commands: Commands,
                     mut food_id: ResMut<FoodId>) {
    let manager = &mut manager.manager;

    let spawn_foods = manager.received::<SpawnFood, SpawnFoodPacketBuilder>(false).unwrap();
    if let Some(spawn_foods) = spawn_foods {
        for food in spawn_foods.iter() {
            spawn_food(&mut commands, food_id.as_mut(), None, food.position.0, food.position.1);
        }
    }
}