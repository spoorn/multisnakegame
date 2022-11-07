use bevy::prelude::*;
use bevy::time::FixedTimestep;
use rand::prelude::random;

const SNAKE_HEAD_COLOR: Color = Color::rgb(0.7, 0.7, 0.7);
const FOOD_COLOR: Color = Color::rgb(1.0, 0.0, 1.0);
const ARENA_WIDTH: u32 = 10;
const ARENA_HEIGHT: u32 = 10;

#[derive(Component)]
struct SnakeHead {
    input_direction: Direction,
    direction: Direction
}

#[derive(Component)]
struct Food;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
struct Position {
    x: i32,
    y: i32,
}

#[derive(Component)]
struct Size {
    width: f32,
    height: f32,
}

impl Size {
    pub fn square(x: f32) -> Self {
        Self { width: x, height: x }
    }
}

#[derive(PartialEq, Copy, Clone, Debug)]
enum Direction {
    Left,
    Up,
    Right,
    Down,
}

impl Direction {
    fn opposite(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
            Self::Up => Self::Down,
            Self::Down => Self::Up,
        }
    }
}

fn size_scaling(windows: Res<Windows>, mut q: Query<(&Size, &mut Transform)>) {
    let window = windows.get_primary().unwrap();
    for (sprite_size, mut transform) in q.iter_mut() {
        transform.scale = Vec3::new(
            sprite_size.width / ARENA_WIDTH as f32 * window.width() as f32,
            sprite_size.height / ARENA_HEIGHT as f32 * window.height() as f32,
            1.0,
        );
    }
}

fn position_translation(windows: Res<Windows>, mut q: Query<(&mut Position, &mut Transform), Changed<Position>>) {
    fn convert(pos: f32, bound_window: f32, bound_game: f32) -> f32 {
        let tile_size = bound_window / bound_game;
        pos / bound_game * bound_window - (bound_window / 2.) + (tile_size / 2.)
    }
    let window = windows.get_primary().unwrap();
    for (mut pos, mut transform) in q.iter_mut() {
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
        
        transform.translation = Vec3::new(
            convert(pos.x as f32, window.width() as f32, ARENA_WIDTH as f32),
            convert(pos.y as f32, window.height() as f32, ARENA_HEIGHT as f32),
            0.0,
        );
    }
}

fn setup_camera(mut commands: Commands) {
    commands.spawn_bundle(Camera2dBundle::default());
}

fn spawn_snake(mut commands: Commands) {
    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite {
                color: SNAKE_HEAD_COLOR,
                ..default()
            },
            ..default()
        })
        .insert(SnakeHead { input_direction: Direction::Right, direction: Direction::Right })
        .insert(Position { x: 3, y: 3 })
        .insert(Size::square(0.8));
}

fn spawn_food(mut commands: Commands) {
    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite {
                color: FOOD_COLOR,
                ..default()
            },
            ..default()
        })
        .insert(Food)
        .insert(Position {
            x: (random::<f32>() * ARENA_WIDTH as f32) as i32,
            y: (random::<f32>() * ARENA_HEIGHT as f32) as i32,
        })
        .insert(Size::square(0.8));
}

fn snake_movement_input(keys: Res<Input<KeyCode>>,  mut head_positions: Query<&mut SnakeHead>) {
    for mut head in head_positions.iter_mut() {
        let dir: Direction = if keys.pressed(KeyCode::Left) {
            Direction::Left
        } else if keys.pressed(KeyCode::Down) {
            Direction::Down
        } else if keys.pressed(KeyCode::Up) {
            Direction::Up
        } else if keys.pressed(KeyCode::Right) {
            Direction::Right
        } else {
            head.input_direction
        };
        if dir != head.direction.opposite() {
            head.input_direction = dir;
        }
    }
}

fn snake_movement(
    mut head_positions: Query<(&mut Position, &mut SnakeHead)>,
) {
    for (mut position, mut head) in head_positions.iter_mut() {
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
    }
}

fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            title: "Snake!".to_string(),
            width: 500.0,
            height: 500.0,
            ..default()
        })
        .insert_resource(ClearColor(Color::rgb(0.04, 0.04, 0.04)))
        .add_startup_system(setup_camera)
        .add_startup_system(spawn_snake)
        .add_plugins(DefaultPlugins)
        .add_system_set(SystemSet::new().with_run_criteria(FixedTimestep::step(0.2)).with_system(snake_movement))
        .add_system(snake_movement_input.before(snake_movement))
        .add_system_set_to_stage(
            CoreStage::PostUpdate,
            SystemSet::new().with_system(position_translation).with_system(size_scaling),
        )
        .add_system_set(SystemSet::new().with_run_criteria(FixedTimestep::step(1.0)).with_system(spawn_food))
        .run();
}
