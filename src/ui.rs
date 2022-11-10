use bevy::prelude::*;
use iyes_loopless::prelude::*;

use crate::state::GameState;
use crate::ui::components::*;
use crate::ui::mainmenu::*;

mod mainmenu;
mod components;

pub struct UiPlugin;

impl Plugin for UiPlugin {

    fn build(&self, app: &mut App) {
        app
            .add_loopless_state(GameState::MainMenu)
            .add_enter_system(GameState::MainMenu, main_menu_setup)
            // Common systems to all screens that handles buttons behaviour
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(GameState::MainMenu)
                    .with_system(menu_action)
                    .with_system(button_system)
                    .into()
            )
            .add_exit_system(GameState::MainMenu, despawn_screen::<OnMainMenuScreen>);
    }
}