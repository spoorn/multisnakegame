use bevy::prelude::*;
use iyes_loopless::prelude::*;

use components::Size;

use crate::common::components::Position;
use crate::common::constants::{ARENA_HEIGHT, ARENA_WIDTH};
use crate::snake::components::SnakeHead;
use crate::snake::spawn_snake;
use crate::state::GameState;

pub mod components;
pub mod constants;

pub struct CommonPlugin {
    pub is_client: bool
}

impl Plugin for CommonPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_startup_system(setup_camera).add_enter_system(GameState::PreGame, pre_game);

        if self.is_client {
            app.add_system_set_to_stage(
                CoreStage::PostUpdate,
                ConditionSet::new()
                    .run_in_state(GameState::Running)
                    .with_system(position_translation)
                    .with_system(size_scaling)
                    .into(),
            );
        }
    }
}

pub fn correct_position_at_ends(mut pos: &mut Position) {
    if pos.x >= ARENA_WIDTH as i32 {
        pos.x = 0;
    } else if pos.x < 0 {
        pos.x = ARENA_WIDTH as i32 - 1;
    }

    if pos.y >= ARENA_HEIGHT as i32 {
        pos.y = 0;
    } else if pos.y < 0 {
        pos.y = ARENA_HEIGHT as i32 - 1;
    }
}

fn size_scaling(windows: Res<Windows>, mut q: Query<(&Size, &mut Transform)>) {
    if let Some(window) = windows.get_primary() {
        for (sprite_size, mut transform) in q.iter_mut() {
            transform.scale = Vec3::new(
                sprite_size.width / ARENA_WIDTH as f32 * window.width() as f32,
                sprite_size.height / ARENA_HEIGHT as f32 * window.height() as f32,
                1.0,
            );
        }
    }
}

fn position_translation(
    windows: Res<Windows>,
    mut q: Query<(&mut Position, &mut Transform, Option<&SnakeHead>)>, /*, Changed<Position>> */
) {
    fn convert(pos: f32, bound_window: f32, bound_game: f32) -> f32 {
        let tile_size = bound_window / bound_game;
        pos / bound_game * bound_window - (bound_window / 2.) + (tile_size / 2.)
    }
    if let Some(window) = windows.get_primary() {
        // Server should correct the position according to arena width/height before sending to client.
        // Client only needs to correct position for any client-tracked positions
        for (mut pos, mut transform, head) in q.iter_mut() {
            let z = if head.is_some() { 1.0 } else { 0.0 };

            transform.translation = Vec3::new(
                convert(pos.x as f32, window.width() as f32, ARENA_WIDTH as f32),
                convert(pos.y as f32, window.height() as f32, ARENA_HEIGHT as f32),
                z,
            );
        }
    }
}

fn setup_camera(mut commands: Commands) {
    commands.spawn_bundle(Camera2dBundle::default());
}

fn pre_game(mut commands: Commands) {
    commands.insert_resource(NextState(GameState::Running));
    spawn_snake(commands);
}
