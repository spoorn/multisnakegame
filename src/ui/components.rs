use bevy::prelude::Component;

// All actions that can be triggered from a button click
#[derive(Component)]
pub enum MenuButtonAction {
    NewGame,
    BackToMainMenu,
    Quit,
}

// Tag component used to tag entities added on the main menu screen
#[derive(Component)]
pub struct OnMainMenuScreen;