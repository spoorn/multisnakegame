use bevy::prelude::Component;

#[derive(Component, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

#[derive(Component)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub fn square(x: f32) -> Self {
        Self { width: x, height: x }
    }
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum Direction {
    Left,
    Up,
    Right,
    Down,
}

impl Direction {
    pub fn opposite(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
            Self::Up => Self::Down,
            Self::Down => Self::Up,
        }
    }
}
