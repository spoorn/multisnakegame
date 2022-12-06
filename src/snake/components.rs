use bevy::prelude::{Component, Entity, SystemLabel, Timer};

use crate::common::components::Direction;

#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemLabel)]
pub enum SnakeState {
    Movement,
    EatFood
}

#[derive(Component)]
pub struct SnakeHead {
    pub id: u8,
    pub input_direction: Direction,
    pub direction: Direction,
    pub tail: Vec<Entity>,
    pub timer: Timer,
}

#[derive(Component)]
pub struct Tail;
