use bevy::prelude::{Component, Entity, SystemLabel, Timer};

use crate::common::components::Direction;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[derive(SystemLabel)]
pub enum SnakeState {
    Movement,
}

#[derive(Component)]
pub struct SnakeHead {
    pub input_direction: Direction,
    pub direction: Direction,
    pub tail: Vec<Entity>,
    pub timer: Timer
}

#[derive(Component)]
pub struct Tail;