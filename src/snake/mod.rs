use std::time::Duration;

use bevy::prelude::*;
use iyes_loopless::prelude::*;

use crate::common::components::{Direction, Position, Size};
use crate::snake::components::{SnakeHead, SnakeState, Tail};
use crate::state::GameState;

pub mod components;
pub mod server;
pub mod client;

pub struct SnakePlugin;

impl Plugin for SnakePlugin {
    fn build(&self, app: &mut App) {
    }
}

const SNAKE_HEAD_COLOR: Color = Color::rgb(0.7, 0.7, 0.7);
const SNAKE_SEGMENT_COLOR: Color = Color::rgb(0.3, 0.3, 0.3);

pub fn spawn_snake(mut commands: Commands) {
    let mut speed_limiter = Timer::from_seconds(0.2, true);
    // Instant tick the timer so snake starts moving immediately when spawned
    speed_limiter.tick(Duration::from_secs_f32(0.2));
    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite {
                color: SNAKE_HEAD_COLOR,
                ..default()
            },
            ..default()
        })
        .insert(SnakeHead {
            input_direction: Direction::Right,
            direction: Direction::Right,
            tail: vec![],
            timer: speed_limiter,
        })
        .insert(Position { x: 3, y: 3 })
        .insert(Size::square(0.8));
}

#[inline]
pub fn spawn_tail(commands: &mut Commands, position: Position) -> Entity {
    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite {
                color: SNAKE_SEGMENT_COLOR,
                ..default()
            },
            ..default()
        })
        .insert(Tail)
        .insert(position)
        .insert(Size::square(0.7))
        .id()
}

pub fn move_snake(time_delta: Duration, mut position: &mut Position, mut head: &mut SnakeHead, mut positions: &mut Query<&mut Position, Without<SnakeHead>>) -> bool {
    let mut res = false;
    if head.timer.finished() {
        // Tail
        for (i, tail) in head.tail.iter().enumerate().rev() {
            if i == 0 {
                let mut pos = positions.get_mut(*tail).unwrap();
                pos.x = position.x;
                pos.y = position.y;
            } else {
                let next_x;
                let next_y;
                // Beat borrow checker
                {
                    let next_pos = positions.get(head.tail[i - 1]).unwrap();
                    next_x = next_pos.x;
                    next_y = next_pos.y;
                }
                let mut pos = positions.get_mut(*tail).unwrap();
                pos.x = next_x;
                pos.y = next_y;
            }
        }

        // Head
        head.direction = head.input_direction;
        match &head.input_direction {
            Direction::Left => {
                position.x -= 1;
            }
            Direction::Up => {
                position.y += 1;
            }
            Direction::Right => {
                position.x += 1;
            }
            Direction::Down => {
                position.y -= 1;
            }
        }
        res = true;
    }

    head.timer.tick(time_delta);
    res
}