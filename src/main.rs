use std::time::Duration;
use bevy::app::AppExit;

use bevy::prelude::*;
use bevy::utils::HashMap;
use iyes_loopless::prelude::*;
use rand::prelude::random;

const SNAKE_HEAD_COLOR: Color = Color::rgb(0.7, 0.7, 0.7);
const SNAKE_SEGMENT_COLOR: Color = Color::rgb(0.3, 0.3, 0.3);
const FOOD_COLOR: Color = Color::rgb(1.0, 0.0, 1.0);
const ARENA_WIDTH: u32 = 20;
const ARENA_HEIGHT: u32 = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum GameState {
    MainMenu,
    Paused,
    PreGame,
    Running
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[derive(SystemLabel)]
enum SnakeState {
    Movement,
}

#[derive(Component)]
struct SnakeHead {
    input_direction: Direction,
    direction: Direction,
    tail: Vec<Entity>,
    timer: Timer
}

#[derive(Component)]
struct Tail;

#[derive(Component)]
struct Food;

#[derive(Component, Clone, Copy, PartialEq, Eq, Hash)]
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
        for (mut pos, mut transform, head) in q.iter_mut() {
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

            let z = if let Some(_) = head { 1.0 } else { 0.0 };

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

fn spawn_snake(mut commands: Commands) {
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
            timer: speed_limiter
        })
        .insert(Position { x: 3, y: 3 })
        .insert(Size::square(0.8));
}

#[inline]
fn spawn_tail(commands: &mut Commands, position: Position) -> Entity {
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

fn snake_movement_input(keys: Res<Input<KeyCode>>, mut head_positions: Query<&mut SnakeHead>) {
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
    time: Res<Time>,
    mut head_positions: Query<(&mut Position, &mut SnakeHead)>,
    mut positions: Query<&mut Position, Without<SnakeHead>>,
) {
    for (mut position, mut head) in head_positions.iter_mut() {
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
        }
        
        head.timer.tick(time.delta());
    }
}

fn eat_food(
    mut commands: Commands,
    foods: Query<(Entity, &Position), With<Food>>,
    mut snakes: Query<(&Position, &mut SnakeHead)>,
    positions: Query<&Position, (Without<SnakeHead>, Without<Food>)>,
) {
    let food_positions = get_food_positions(foods);

    for (position, mut head) in snakes.iter_mut() {
        if let Some(entity) = food_positions.get(position) {
            commands.entity(*entity).despawn();
            let mut position = position;
            if !head.tail.is_empty() {
                position = positions.get(*head.tail.last().unwrap()).unwrap();
            }
            head.tail.push(spawn_tail(&mut commands, position.clone()));
        }
    }
}

#[inline]
fn get_food_positions(foods: Query<(Entity, &Position), With<Food>>) -> HashMap<Position, Entity> {
    let mut food_positions = HashMap::new();
    // Assumes no position has multiple food
    for (entity, position) in foods.iter() {
        food_positions.insert(position.clone(), entity);
    }
    food_positions
}

// Tag component used to tag entities added on the main menu screen
#[derive(Component)]
struct OnMainMenuScreen;

const TEXT_COLOR: Color = Color::rgb(0.9, 0.9, 0.9);
const NORMAL_BUTTON: Color = Color::rgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::rgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::rgb(0.35, 0.75, 0.35);

// All actions that can be triggered from a button click
#[derive(Component)]
enum MenuButtonAction {
    NewGame,
    BackToMainMenu,
    Quit,
}

type UiSize = bevy::ui::Size<Val>;

fn main_menu_setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let default_font = asset_server.load("fonts/FiraSans-Bold.ttf");
    // Common style for all buttons on the screen
    let button_style = Style {
        size: UiSize::new(Val::Px(250.0), Val::Px(65.0)),
        margin: UiRect::all(Val::Px(20.0)),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    };
    let button_text_style = TextStyle {
        font: default_font.clone(),
        font_size: 40.0,
        color: TEXT_COLOR,
    };
    
    commands.spawn_bundle(NodeBundle {
        style: Style {
            margin: UiRect::all(Val::Auto),
            flex_direction: FlexDirection::ColumnReverse,
            align_items: AlignItems::Center,
            ..default()
        },
        color: Color::SEA_GREEN.into(),
        ..default()
    })
        .insert(OnMainMenuScreen)
        .with_children(|parent| {
            // Display the game name
            parent.spawn_bundle(
                TextBundle::from_section(
                    "Snake Game",
                    TextStyle {
                        font: default_font.clone(),
                        font_size: 80.0,
                        color: TEXT_COLOR,
                    },
                )
                    .with_style(Style {
                        margin: UiRect::all(Val::Px(50.0)),
                        ..default()
                    }),
            );

            parent
                .spawn_bundle(ButtonBundle {
                    style: button_style.clone(),
                    color: NORMAL_BUTTON.into(),
                    ..default()
                })
                .insert(MenuButtonAction::NewGame)
                .with_children(|parent| {
                    parent.spawn_bundle(TextBundle::from_section(
                        "New Game",
                        button_text_style.clone(),
                    ));
                });
        });
}

// This system handles changing all buttons color based on mouse interaction
fn button_system(
    mut interaction_query: Query<(&Interaction, &mut UiColor), (Changed<Interaction>, With<Button>)>,
) {
    for (interaction, mut color) in &mut interaction_query {
        *color = match *interaction {
            Interaction::Clicked => PRESSED_BUTTON.into(),
            Interaction::Hovered => HOVERED_BUTTON.into(),
            Interaction::None => NORMAL_BUTTON.into(),
        }
    }
}

fn menu_action(
    mut commands: Commands,
    interaction_query: Query<(&Interaction, &MenuButtonAction), (Changed<Interaction>, With<Button>)>,
    mut app_exit_events: EventWriter<AppExit>,
) {
    for (interaction, menu_button_action) in &interaction_query {
        if *interaction == Interaction::Clicked {
            match menu_button_action {
                MenuButtonAction::NewGame => {
                    commands.insert_resource(NextState(GameState::PreGame))
                }
                MenuButtonAction::BackToMainMenu => {
                    commands.insert_resource(NextState(GameState::MainMenu))
                }
                MenuButtonAction::Quit => {
                    app_exit_events.send(AppExit)
                }
            }
        }
    }
}

// Generic system that takes a component as a parameter, and will despawn all entities with that component
fn despawn_screen<T: Component>(to_despawn: Query<Entity, With<T>>, mut commands: Commands) {
    for entity in &to_despawn {
        commands.entity(entity).despawn_recursive();
    }
}

fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            title: "Snake!".to_string(),
            width: 1000.0,
            height: 1000.0,
            // TODO: always opens on primary monitor
            position: WindowPosition::Centered(MonitorSelection::Primary),
            ..default()
        })
        .insert_resource(ClearColor(Color::rgb(0.04, 0.04, 0.04)))
        .add_loopless_state(GameState::MainMenu)
        .add_startup_system(setup_camera)
        .add_enter_system(GameState::PreGame, pre_game)
        .add_plugins(DefaultPlugins)
        .add_system(snake_movement.run_in_state(GameState::Running).label(SnakeState::Movement))
        .add_system(eat_food.run_in_state(GameState::Running).after(SnakeState::Movement))
        .add_system(snake_movement_input.run_in_state(GameState::Running).after(SnakeState::Movement))
        .add_system_set_to_stage(
            CoreStage::PostUpdate,
            ConditionSet::new().run_in_state(GameState::Running).with_system(position_translation).with_system(size_scaling).into(),
        )
        .add_fixed_timestep(Duration::from_secs(1), "spawn_food")
        .add_fixed_timestep_system("spawn_food", 0, spawn_food.run_in_state(GameState::Running))
        .add_enter_system(GameState::MainMenu, main_menu_setup)
        // Common systems to all screens that handles buttons behaviour
        .add_system_set(
            ConditionSet::new()
                .run_in_state(GameState::MainMenu)
                .with_system(menu_action)
                .with_system(button_system)
                .into()
        )
        .add_exit_system(GameState::MainMenu, despawn_screen::<OnMainMenuScreen>)
        .run();
}
