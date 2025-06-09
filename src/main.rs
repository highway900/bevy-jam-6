mod audio;
mod collision_system;
mod models;

use std::f32::consts::{FRAC_PI_2, PI};

use bevy::{
    color::palettes::{
        css::{GOLD, YELLOW},
        tailwind::CYAN_200,
    },
    pbr::NotShadowCaster,
    platform::collections::HashMap,
    prelude::*,
    render::camera::ScalingMode,
};

use bevy_asset_loader::loading_state::{
    LoadingState, LoadingStateAppExt, config::ConfigureLoadingState,
};
use bevy_common_assets::ron::RonAssetPlugin;
// use bevy_inspector_egui::prelude::*;
use collision_system::{
    CollisionEvent, LLAabb3d, collision_detection_system, player_collision_handling_system,
};
use models::ModelAssets;

// Constants for animation durations
const PLAYER_ACTION_ANIMATION_DURATION: f32 = 0.44; // e.g., move or jump start
const GAME_MOVE_ANIMATION_DURATION: f32 = 0.47; // e.g., environment changes, enemies move
const PLAYER_JUMP_ANIMATION_DURATION: f32 = 0.39; // e.g., landing animation
const PLAYER_JUMP_LAND_ANIMATION_DURATION: f32 = 0.42; // e.g., landing animation
const SEQUENCE_LENGTH: usize = 8;
const BIRD_Y: i32 = 2;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum AppState {
    #[default]
    AssetLoading,
    // Menu,
    InGame,
    EndGame,
    WinGame,
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, SubStates)]
#[source(AppState = AppState::InGame)]
enum GameState {
    #[default]
    PlayerIdle, // Player is choosing an action
    PlayerActionInProgress, // Player's chosen action (move or jump start) is "animating"
    GameTurnInProgress,     // Game's action is "animating"
    PlayerFinishingJump,    // Player is "animating" landing the jump
    Paused,
}

#[derive(Debug, Component, PartialEq)]
#[allow(dead_code)]
enum GameObjectType {
    Player,
    Log,
    Rock,
}

#[derive(Debug, Event, PartialEq)]
pub enum GameEvent {
    Over,
    Win,
}

impl GameEvent {
    pub fn text(&self) -> String {
        match self {
            GameEvent::Over => "GAME OVER".to_string(),
            GameEvent::Win => "ALL BIRDS RESCUED".to_string(),
        }
    }
}

// Resource to track what kind of action the player is taking,
// specifically if they are in the process of a jump.
#[derive(Resource, Default, Debug)]
struct PlayerActionTracker {
    is_moving: Direction,
    is_jumping: bool,
}

// Resource to act as a shared timer for actions.
// Different states will set its duration and wait for it.
#[derive(Resource, Debug)]
struct ActionTimer(Timer);

impl Default for ActionTimer {
    fn default() -> Self {
        ActionTimer(Timer::from_seconds(1.0, TimerMode::Once))
    }
}

#[derive(Clone, Reflect, serde::Deserialize, Asset)]
pub struct Level {
    pub seq: Vec<LogSequence>,
    pub bird_map: HashMap<IVec3, Entity>,
}

#[derive(Clone, Reflect, Default, serde::Deserialize)]
pub struct LogSequence {
    /// This value is used to mod with the step to get the nth step to spawn
    pub log_sequence_nth_step: [u32; SEQUENCE_LENGTH],
    /// How many steps should the log sequence spawn for
    log_sequence_step_length: u32,
    pub spawn_offset_x: [i32; SEQUENCE_LENGTH],
    pub spawn_offset_z: [i32; SEQUENCE_LENGTH],
    pub spawn_offset_x_base: i32,
    pub sequence_index: usize,
    pub sequence_step_counter: u32,
}

impl LogSequence {
    /// increment the sequence index and wrap if its reached the end
    // pub fn seq_next(&mut self) {
    //     // Increment the sequence index and wrap it if it reaches the end.
    //     // The length of the array `log_sequence_nth_step` is fixed at SEQUENCE_LENGTH (8),
    //     // which is guaranteed to be non-zero, so the modulo operation is safe.
    //     self.sequence_index = (self.sequence_index + 1) % self.log_sequence_nth_step.len();
    // }

    pub fn seq_idx(&self, step: u32) -> usize {
        step as usize % self.log_sequence_nth_step.len()
    }

    pub fn check_seq(&self, step: u32, mod_val: u32) -> bool {
        let idx: usize = self.seq_idx(step);
        step % self.log_sequence_nth_step[idx] == mod_val
    }

    pub fn get_nth_step(&self) -> u32 {
        self.log_sequence_nth_step[self.sequence_index]
    }

    pub fn get_step_length(&self) -> u32 {
        self.log_sequence_step_length
    }

    pub fn get_spawn_offset_x(&self) -> i32 {
        self.spawn_offset_x_base + self.spawn_offset_x[self.sequence_index]
    }

    pub fn get_spawn_offset_z(&self) -> i32 {
        self.spawn_offset_z[self.sequence_index]
    }

    pub fn debug_print(&self) {
        info!(">>>> Spawning log");
        info!(
            "\nseq:         [{}]\nseq offx:    [{}]\nseq offz:    [{}]\nseq_idx:     [{}]\nseq_nth:     [{}]\noffsetx:     [{}]\noffsetz:     [{}]",
            format_array_with_bracket(&self.log_sequence_nth_step, self.sequence_index),
            format_array_with_bracket(&self.spawn_offset_x, self.sequence_index),
            format_array_with_bracket(&self.spawn_offset_z, self.sequence_index),
            self.sequence_index,
            self.get_nth_step(),
            self.get_spawn_offset_x(),
            self.get_spawn_offset_z()
        );
        info!("<<<< Spawning log");
    }
}

// Should be a feature InspectorOptions
#[derive(Resource, Clone, Reflect, serde::Deserialize, Asset)]
#[reflect(Resource)]
pub struct Game {
    pub board_size_x: u32,
    pub board_size_y: u32,
    pub start: IVec2,
    pub player_pos: IVec3,
    pub current_step: u32,
    current_level: usize,
    pub levels: Vec<Level>,
    pub bevy_count: u32,
}

const LOG1_OFFSET_X: i32 = 2;
const LOG2_OFFSET_X: i32 = 6;
const LOG3_OFFSET_X: i32 = 10;

impl Default for Game {
    fn default() -> Self {
        Self {
            board_size_x: 12,
            board_size_y: 12,
            start: IVec2::new(3, 0),
            current_step: 0,
            player_pos: IVec3::new(3, 0, 0),
            current_level: 0,
            bevy_count: 0,
            #[rustfmt::skip]
            levels: vec![Level {
                seq: vec![
                    LogSequence {
                        log_sequence_step_length: 2,
                        log_sequence_nth_step:      [1, 0, 1, 1, 1, 0, 0, 0],
                        spawn_offset_z:             [0, 1, 0, 0, 0, 0, 0, 0],
                        spawn_offset_x:             [0, 1, 0, 0, 1, 0, 0, 0],
                        spawn_offset_x_base: LOG1_OFFSET_X,
                        ..default()
                    },
                    LogSequence {
                        log_sequence_step_length: 2,
                        log_sequence_nth_step:      [0, 1, 1, 0, 1, 0, 0, 0],
                        spawn_offset_z:             [0, 0, 0, 0, 0, 0, 0, 0],
                        spawn_offset_x:             [0, 1, 0, 0, 1, 0, 0, 0],
                        spawn_offset_x_base: LOG2_OFFSET_X,
                        ..default()
                    },
                    LogSequence {
                        log_sequence_step_length: 2,
                        log_sequence_nth_step:      [1, 0, 1, 1, 0, 1, 0, 1],
                        spawn_offset_z:             [0, 0, 1, 0, 0, 0, 0, 0],
                        spawn_offset_x:             [0, 0, 0, 0, 0, 0, 0, 0],
                        spawn_offset_x_base: LOG3_OFFSET_X,
                        ..default()
                    },
                ],
                bird_map: HashMap::from_iter([
                    (IVec3::new(7,BIRD_Y,2), Entity::PLACEHOLDER),
                    (IVec3::new(8,BIRD_Y,7), Entity::PLACEHOLDER),
                    (IVec3::new(3,BIRD_Y,5), Entity::PLACEHOLDER),
                ])
            }],
        }
    }
}

impl Game {
    pub fn board_size_as_vec3(&self) -> Vec3 {
        Vec3::new(self.board_size_x as f32, 0.0, self.board_size_y as f32)
    }

    pub fn board_size_as_ivec2(&self) -> UVec2 {
        UVec2::new(self.board_size_x, self.board_size_y)
    }

    pub fn is_valid_player_move(&self, dir: &Direction) -> bool {
        let pos = self.player_pos + dir.value();
        // Assuming board starts at (0, 0) in the XZ plane
        if pos.x >= 0
            && pos.x < self.board_size_x as i32
            && pos.z >= 0
            && pos.z < self.board_size_y as i32
        {
            true
        } else {
            println!("outside boundary: {pos:?}");
            false
        }
    }

    pub fn is_valid_board_pos(&self, pos: IVec3, dir: &Direction) -> bool {
        let pos = pos + dir.value();
        // Assuming board starts at (0, 0) in the XZ plane
        if pos.x > -1
            && pos.x <= self.board_size_x as i32
            && pos.z > -1
            && pos.z <= self.board_size_y as i32
        {
            true
        } else {
            println!("outside boundary: {pos:?}");
            false
        }
    }

    pub fn current_level_mut(&mut self) -> &mut Level {
        return &mut self.levels[self.current_level];
    }

    pub fn current_level(&self) -> &Level {
        return &self.levels[self.current_level];
    }
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, RonAssetPlugin::<Level>::new(&["level.ron"])))
        .init_state::<AppState>()
        .add_sub_state::<GameState>()
        .add_loading_state(
            LoadingState::new(AppState::AssetLoading)
                .continue_to_state(AppState::InGame)
                .load_collection::<ModelAssets>(),
        )
        .init_resource::<PlayerActionTracker>()
        .init_resource::<ActionTimer>()
        .init_resource::<PrevState>()
        .init_resource::<DebugSkipPlayerAction>()
        .add_event::<PlayerBirdRescueEvent>()
        .add_event::<CollisionEvent>()
        .add_event::<GameMessage>()
        .insert_resource(ClearColor(Color::srgb(0.5, 0.5, 0.9)))
        .add_systems(Startup, setup_initial_app_state) // Go directly to InGame for this demo
        // Startup and AppState transitions
        .add_systems(
            OnEnter(AppState::InGame),
            (text_update_game_message_hide, setup_game_environment).chain(),
        )
        // GameState: PlayerIdle
        .add_systems(
            OnEnter(GameState::PlayerIdle),
            (
                player_idle_entry_message,
                spawn_logs.after(advance_game_turn),
            ),
        )
        .add_systems(
            Update,
            (handle_player_input, no_shadow_bird_on_gltf_butcher)
                .run_if(in_state(GameState::PlayerIdle)),
        )
        // GameState: PlayerActionInProgress
        .add_systems(
            OnEnter(GameState::PlayerActionInProgress),
            setup_player_action_timer,
        )
        .add_systems(
            Update,
            (process_player_action, update_player_pos_after_jump)
                .chain()
                .run_if(in_state(GameState::PlayerActionInProgress)),
        )
        .add_systems(
            OnExit(GameState::PlayerActionInProgress),
            player_check_for_bird,
        )
        // GameState: GameTurnInProgress
        .add_systems(
            OnEnter(GameState::GameTurnInProgress),
            setup_game_turn_timer,
        )
        .add_systems(
            Update,
            (process_game_turn, roll_logs, collision_detection_system)
                .chain()
                .run_if(in_state(GameState::GameTurnInProgress)),
        )
        // GameState: PlayerFinishingJump
        .add_systems(
            OnEnter(GameState::PlayerFinishingJump),
            setup_player_jump_land_timer,
        )
        .add_systems(
            Update,
            process_player_finishing_jump.run_if(in_state(GameState::PlayerFinishingJump)),
        )
        .add_systems(OnExit(GameState::GameTurnInProgress), advance_game_turn)
        // Win game
        .add_systems(
            Update,
            (roll_logs, update_win_timer)
                .chain()
                .run_if(in_state(AppState::WinGame)),
        )
        .add_systems(OnExit(AppState::WinGame), cleanup_game)
        // End game
        .add_systems(
            Update,
            (
                endgame_message_update,
                player_end_handling_system,
                roll_logs,
            )
                .chain()
                .run_if(in_state(AppState::EndGame)),
        )
        .add_systems(OnExit(AppState::EndGame), cleanup_game)
        // Systems running all the time during a game
        .add_systems(
            Update,
            (
                rotate_system,
                update_aabb_system,
                draw_aabb_gizmos,
                player_collision_handling_system,
                toggle_debug_skip_player_action,
                text_update_bird_count,
            )
                .chain()
                .run_if(in_state(AppState::InGame)),
        )
        // Pause handling
        .add_systems(OnEnter(GameState::Paused), setup_paused_screen)
        .add_systems(OnExit(GameState::Paused), cleanup_paused_screen)
        .add_systems(
            Update,
            toggle_pause.run_if(in_state(AppState::InGame)), // Pause can be triggered anytime InGame
        )
        // Logging for state transitions
        .add_systems(Update, log_gamestate_transitions)
        .add_observer(player_wins_trigger)
        .add_observer(increment_bevy)
        .add_observer(text_update_game_message)
        .run();
}

const TILE_SIZE: f32 = 1.0;
const TILE_HALF_SIZE: f32 = 0.5;

#[derive(Component)]
pub struct Player;

#[derive(Component)]
pub struct PlayerEnd(pub Timer);

#[derive(Component)]
pub struct PlayerWin(pub Timer);

#[derive(Component)]
pub struct Log;

#[derive(Component)]
pub struct Tile;

#[derive(Component)]
pub struct Bird;

#[derive(Component)]
pub struct PlayerCursor;

#[derive(Component, Event)]
pub struct GameMessage;

#[derive(Component)]
pub struct BirdCountText;

#[derive(Clone, Reflect, Debug, PartialEq, Default)]
pub enum Direction {
    #[default]
    None,
    North,
    East,
    South,
    West,
}

const DIST: i32 = 1;

impl Direction {
    pub fn value(&self) -> IVec3 {
        match self {
            Direction::North => IVec3::new(0, 0, DIST),
            Direction::East => IVec3::new(DIST, 0, 0),
            Direction::South => IVec3::new(0, 0, -DIST),
            Direction::West => IVec3::new(-DIST, 0, 0),
            Direction::None => IVec3::ZERO,
        }
    }
}

fn setup_initial_app_state(
    mut commands: Commands,
    mut next_app_state: ResMut<NextState<AppState>>,
) {
    info!("Startup: Transitioning from Menu (default) to InGame.");
    let game = Game::default();

    commands.insert_resource(AmbientLight {
        brightness: 80.0,
        color: Color::WHITE,
        ..Default::default()
    });

    let light_position = Vec3::new(-3.5, 5.0, 3.5);
    let light_intensity_watts = 1000.0;
    let look_at = game.board_size_as_vec3() * 0.5;
    commands.spawn((
        Name::new("light_sun"),
        DirectionalLight {
            illuminance: light_intensity_watts, // Adjust as needed
            shadows_enabled: true,              // Enable shadows for better depth perception
            color: Color::from(YELLOW),
            ..default()
        },
        Transform::from_translation(light_position).looking_at(look_at, Vec3::Y),
    ));

    commands.spawn((
        Name::new("main_camera"),
        Camera3d::default(),
        Transform::from_translation(Vec3::new(-3., 5.8, -3.5)).looking_at(look_at, Vec3::Y),
        Camera {
            hdr: false,
            ..default()
        },
        Projection::from(OrthographicProjection {
            // 6 world units per pixel of window height.
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: 12.0,
            },
            near: -5.0,
            ..OrthographicProjection::default_3d()
        }),
    ));

    commands
        .spawn((
            Text::new("Rescued Birds: "),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(12.0),
                left: Val::Px(12.0),
                ..default()
            },
            TextFont {
                font_size: 24.0,
                ..default()
            },
        ))
        .with_child((
            TextSpan::new("0"),
            (
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(GOLD.into()),
            ),
            BirdCountText,
        ));

    commands.spawn((
        Text::new(""),
        Node {
            position_type: PositionType::Absolute,
            // Use margin: Val::Auto on all sides to center an absolutely positioned element
            // within its parent (which is typically the root UI node filling the screen).
            left: Val::Auto,
            right: Val::Auto,
            top: Val::Auto,
            bottom: Val::Auto,
            margin: UiRect::all(Val::Auto),
            ..default()
        },
        GameMessage,
        Visibility::Hidden,
    ));

    commands.insert_resource(game);

    // In a real game, you might have a menu system here.
    // For this example, we'll jump straight to InGame.
    next_app_state.set(AppState::AssetLoading);
}

fn setup_game_environment(
    mut game: ResMut<Game>,
    model_assets: Res<ModelAssets>,
    assets_gltf: Res<Assets<Gltf>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut next_game_state: ResMut<NextState<GameState>>,
) {
    info!("Entered AppState::InGame. Setting initial GameState to PlayerIdle.");

    for y in 0..game.board_size_y {
        for x in 0..game.board_size_x {
            commands.spawn((
                Tile,
                Transform::from_xyz(x as f32, 0.0, y as f32),
                Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(TILE_HALF_SIZE)))),
                MeshMaterial3d(materials.add(Color::linear_rgb(0.51, 0.54, 0.075))),
                ShowAabbGizmo {
                    color: Some(Color::linear_rgba(0.51, 0.34, 0.075, 0.75)),
                },
            ));
        }
    }

    let Some(bg_gltf) = assets_gltf.get(&model_assets.background.clone()) else {
        return;
    };
    let Some(bird_gltf) = assets_gltf.get(&model_assets.bird.clone()) else {
        return;
    };

    // background model
    commands.spawn((
        Transform::from_xyz(0.0, 0.0, 0.0)
            .with_rotation(Quat::from_rotation_y(-FRAC_PI_2))
            .with_scale(Vec3::splat(0.5)),
        SceneRoot(bg_gltf.scenes[0].clone()),
    ));

    // birds
    for (k, v) in game.current_level_mut().bird_map.iter_mut() {
        let entity = commands
            .spawn((
                Bird,
                Transform::from_translation(k.as_vec3())
                    .with_rotation(Quat::from_rotation_y(PI))
                    .with_scale(Vec3::splat(0.2)),
                SceneRoot(bird_gltf.scenes[0].clone()),
            ))
            .with_children(|parent| {
                // Shadow
                parent.spawn((
                    Mesh3d(meshes.add(Sphere::new(1.0))),
                    MeshMaterial3d(materials.add(Color::linear_rgb(0.1, 0.0, 0.1))),
                    Transform::from_xyz(0.0, -10.0, 0.0).with_scale(Vec3::new(1.0, 0.2, 1.0)),
                ));
            })
            .id();
        *v = entity;
    }

    // Player
    commands
        .spawn((
            Transform::from_xyz(6.0, TILE_HALF_SIZE, 0.0),
            Player,
            Mesh3d(meshes.add(Cuboid::new(
                TILE_HALF_SIZE,
                TILE_HALF_SIZE * 2.2,
                TILE_HALF_SIZE,
            ))),
            MeshMaterial3d(materials.add(Color::linear_rgb(0.8, 0.1, 0.1))),
            LLAabb3d::new(
                Vec3::new(2.0, TILE_HALF_SIZE, 0.0),
                Vec3::new(
                    TILE_HALF_SIZE * 0.5,
                    TILE_HALF_SIZE * 1.1,
                    TILE_HALF_SIZE * 0.5,
                ),
            ),
            LLShowAabbGizmo {
                color: Some(Color::linear_rgba(0.51, 0.34, 0.075, 0.75)),
            },
            GameObjectType::Player,
        ))
        .with_children(|parent| {
            // Nose (direction)
            parent.spawn((
                Transform::from_xyz(0.0, TILE_HALF_SIZE - 0.1, TILE_HALF_SIZE),
                Mesh3d(meshes.add(Cuboid::new(0.05, 0.05, 0.2))),
                MeshMaterial3d(materials.add(Color::linear_rgb(1., 0.875, 0.545))),
                Rotate::default(),
            ));
        });

    // This system runs when AppState::InGame is entered.
    // We initialize our sub-state machine here.
    next_game_state.set(GameState::PlayerIdle);
    // You could also spawn game entities, UI, etc.
}

fn no_shadow_bird_on_gltf_butcher(
    birds: Query<Entity, With<Bird>>,
    children: Query<&Children>,
    mut commands: Commands,
) {
    for entity in birds.iter() {
        commands.entity(entity).insert_if_new(NotShadowCaster);
        for child in children.iter_descendants(entity) {
            commands.entity(child).insert_if_new(NotShadowCaster);
        }
    }
}

const PALETTE: [&'static str; SEQUENCE_LENGTH] = [
    "#FF8383", "#FFF574", "#A1D6CB", "#A19AD3", "#ca5a2e", "#FFF574", "#A1D6CB", "#A19AD3",
];

pub fn hex_to_color(hex: &str) -> Color {
    match hex_to_srgb(hex) {
        Ok((r, g, b)) => Color::srgb(r, g, b),
        Err(_) => Color::BLACK,
    }
}

fn hex_to_srgb(hex: &str) -> Result<(f32, f32, f32), String> {
    // Remove the leading '#' if present
    let hex = hex.trim_start_matches('#');

    // Ensure the hex string is exactly 6 characters
    if hex.len() != 6 {
        return Err(format!("Invalid hex color code length: {}", hex.len()));
    }

    // Parse the hex string into RGB components
    let r =
        u8::from_str_radix(&hex[0..2], 16).map_err(|e| format!("Invalid red component: {}", e))?;
    let g = u8::from_str_radix(&hex[2..4], 16)
        .map_err(|e| format!("Invalid green component: {}", e))?;
    let b =
        u8::from_str_radix(&hex[4..6], 16).map_err(|e| format!("Invalid blue component: {}", e))?;

    // Convert to normalized sRGB values (0.0 to 1.0)
    let r_srgb = r as f32 / 255.0;
    let g_srgb = g as f32 / 255.0;
    let b_srgb = b as f32 / 255.0;

    Ok((r_srgb, g_srgb, b_srgb))
}

fn format_array_with_bracket<T: std::fmt::Display>(arr: &[T], index: usize) -> String {
    let mut parts = Vec::new();
    for (i, value) in arr.iter().enumerate() {
        if i == index {
            parts.push(format!("[{}]", value));
        } else {
            parts.push(format!("{}", value));
        }
    }
    parts.join(", ")
}

fn spawn_logs(
    mut commands: Commands,
    mut game: ResMut<Game>,
    // mut meshes: ResMut<Assets<Mesh>>,
    // mut materials: ResMut<Assets<StandardMaterial>>,
    model_assets: Res<ModelAssets>,
    assets_gltf: Res<Assets<Gltf>>,
) {
    let current_level = game.current_level;
    let current_step = game.current_step;
    let board_size_y = game.board_size_y;
    let Some(log_gltf) = assets_gltf.get(&model_assets.log.clone()) else {
        return;
    };
    for seq in game.levels[current_level].seq.iter_mut() {
        // check for every nth step
        let seq_idx = seq.seq_idx(current_step);
        info!("<<<< step: {}", current_step);
        if seq.log_sequence_nth_step[seq_idx as usize] > 0 {
            // update the sequence index
            seq.sequence_index = seq.seq_idx(current_step);
            seq.debug_print();

            let top_of_board_minus_offsetz = board_size_y as f32 - seq.get_spawn_offset_z() as f32;

            let log_center = Vec3::new(
                seq.get_spawn_offset_x() as f32 - TILE_HALF_SIZE,
                TILE_HALF_SIZE * 0.5,
                top_of_board_minus_offsetz,
            );
            commands.spawn((
                Log,
                Transform::from_translation(log_center.clone()).with_scale(Vec3::new(
                    TILE_SIZE * 2.0,
                    TILE_HALF_SIZE,
                    TILE_HALF_SIZE,
                )),
                LLAabb3d::new(
                    log_center,
                    Vec3::new(TILE_SIZE * 2.0, TILE_HALF_SIZE * 0.5, TILE_HALF_SIZE * 0.5),
                ),
                // LLShowAabbGizmo {
                //     color: Some(Color::linear_rgba(0.51, 0.34, 0.075, 0.75)),
                // },
                SceneRoot(log_gltf.scenes[0].clone()),
                // Mesh3d(meshes.add(Cuboid::new(TILE_SIZE * 4.0, TILE_HALF_SIZE, TILE_HALF_SIZE))),
                // MeshMaterial3d(materials.add(hex_to_color(PALETTE[seq_idx as usize]))),
                GameObjectType::Log,
            ));
        }
        info!(">>>> seq finished: {}", seq.sequence_index);
    }
}

fn advance_game_turn(mut game: ResMut<Game>) {
    game.current_step += 1;
    info!("Advancing to turn number: {}", game.current_step);
}

fn endgame_message_update(mut text: Single<&mut Text, With<GameMessage>>) {
    text.clear();
    text.push_str("GAME OVER");
}

fn player_end_handling_system(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &mut PlayerEnd), (With<Player>, With<PlayerEnd>)>,
    time: Res<Time>,
    mut next_app_state: ResMut<NextState<AppState>>,
) {
    for (entity, mut tform, mut timer) in query.iter_mut() {
        timer.0.tick(time.delta());
        const SPEED: f32 = 4.5;
        tform.translation.z -= SPEED * timer.0.elapsed().as_secs_f32();
        tform.translation.y += SPEED * 0.5 * timer.0.elapsed().as_secs_f32();
        if let Ok(dir) = Dir3::from_xyz(0.3, 0.8, 0.56) {
            tform.rotate_axis(dir, SPEED * timer.0.elapsed().as_secs_f32() * PI);
        }
        if timer.0.finished() {
            commands.entity(entity).despawn();
            next_app_state.set(AppState::InGame);
        }
    }
}

pub fn player_wins_trigger(
    trigger: Trigger<GameEvent>,
    mut query: Query<Entity, With<Player>>,
    mut next_app_state: ResMut<NextState<AppState>>,
    mut commands: Commands,
) {
    match trigger.event() {
        GameEvent::Win => {
            info!("winner!");
            let Ok(entity) = query.single_mut() else {
                return;
            };
            commands
                .entity(entity)
                .insert_if_new(PlayerWin(Timer::from_seconds(2.6, TimerMode::Once)));

            next_app_state.set(AppState::WinGame);
        }
        _ => (),
    }
}

fn update_win_timer(
    mut query: Query<&mut PlayerWin>,
    time: Res<Time>,
    mut next_app_state: ResMut<NextState<AppState>>,
    mut game: ResMut<Game>,
) {
    for mut timer in query.iter_mut() {
        if timer.0.tick(time.delta()).finished() {
            // This should be where we advance to the next level
            *game = Game::default();
            next_app_state.set(AppState::InGame);
        }
    }
}

fn cleanup_game(
    mut commands: Commands,
    game_entities: Query<
        Entity,
        Or<(
            With<Player>,
            With<Log>,
            With<Tile>,
            With<PlayerCursor>,
            With<Bird>,
        )>,
    >,
    mut game: ResMut<Game>,
) {
    for entity in game_entities.iter() {
        commands.entity(entity).despawn();
    }
    info!("reseting game");
    *game = Game::default();
}

fn update_aabb_system(mut query: Query<(&Transform, &mut LLAabb3d)>) {
    for (transform, mut aabb) in query.iter_mut() {
        // Update the AABB's center to match the entity's translation.
        // Assumes the LLAabb3d was initially defined with its center at the entity's origin (0,0,0)
        // when the entity's transform was also at (0,0,0).
        // If the mesh or visual representation's center is offset from the entity's origin,
        // this update might need adjustment (e.g., `transform.translation + aabb.initial_offset`).
        // For this example, we assume the entity origin is the AABB center.
        aabb.update_center(transform.translation);
    }
}

// Component to mark entities for which AABB gizmos should be drawn
#[derive(Component)]
pub struct LLShowAabbGizmo {
    pub color: Option<Color>,
}

// System to draw debug gizmos for Aabb3d components
fn draw_aabb_gizmos(mut gizmos: Gizmos, query: Query<(&LLAabb3d, Option<&LLShowAabbGizmo>)>) {
    for (aabb, show_gizmo) in query.iter() {
        // Only draw if the ShowAabbGizmo component is present
        if let Some(show) = show_gizmo {
            let color = show.color.unwrap_or(CYAN_200.into()); // Use specified color or a default debug color
            // Create a transform for the gizmo based on the AABB center and size
            let gizmo_transform =
                Transform::from_translation(aabb.center()).with_scale(aabb.half_extents() * 2.0); // Scale cuboid by full extent

            // Draw the cuboid gizmo
            gizmos.cuboid(gizmo_transform, color);
        }
    }
}

fn player_idle_entry_message() {
    info!("GameState: PlayerIdle. Press 'M' for Move, 'J' for Jump.");
}

fn handle_player_input(
    input: Res<ButtonInput<KeyCode>>,
    mut next_game_state: ResMut<NextState<GameState>>,
    mut player_action_tracker: ResMut<PlayerActionTracker>,
    debug: Res<DebugSkipPlayerAction>,
) {
    let mut moving: bool = false;
    let mut jump: bool = false;
    if input.just_pressed(KeyCode::KeyW) {
        moving = true;
        player_action_tracker.is_moving = Direction::North;
    } else if input.just_pressed(KeyCode::KeyD) {
        moving = true;
        player_action_tracker.is_moving = Direction::West;
    } else if input.just_pressed(KeyCode::KeyS) {
        moving = true;
        player_action_tracker.is_moving = Direction::South;
    } else if input.just_pressed(KeyCode::KeyA) {
        moving = true;
        player_action_tracker.is_moving = Direction::East;
    } else if input.just_pressed(KeyCode::KeyJ) {
        jump = true;
        info!("Player choo choo chooses to JUMP.");
        player_action_tracker.is_jumping = true;
    };

    if input.just_released(KeyCode::Escape) {
        std::process::exit(0)
    }

    if moving {
        info!("Player chooses MOVE.");
        player_action_tracker.is_jumping = false;
    };
    if moving | jump | debug.skip_player_action {
        next_game_state.set(GameState::PlayerActionInProgress);
    };
}

// Component for player movement animation
#[derive(Component)]
pub struct PlayerMove {
    pub start_position: Vec3,
    pub target_position: Vec3,
}

fn setup_player_action_timer(
    mut action_timer: ResMut<ActionTimer>,
    player_action_tracker: Res<PlayerActionTracker>,
    mut commands: Commands,
    player_query: Query<(Entity, &Transform), With<Player>>,
) {
    let action_type = if player_action_tracker.is_jumping {
        "JUMP START"
    } else {
        "MOVE"
    };
    info!(
        "GameState: PlayerActionInProgress ({action_type}). Starting {PLAYER_ACTION_ANIMATION_DURATION}s animation timer."
    );
    action_timer.0 = if player_action_tracker.is_jumping {
        Timer::from_seconds(PLAYER_JUMP_ANIMATION_DURATION, TimerMode::Once)
    } else {
        Timer::from_seconds(PLAYER_ACTION_ANIMATION_DURATION, TimerMode::Once)
    };

    action_timer.0.reset(); // Ensure it's fresh

    // Set up player movement animation
    if let Ok((player_entity, player_transform)) = player_query.single() {
        let start_position = player_transform.translation;
        let target_position = if player_action_tracker.is_jumping {
            // Jump upward
            Vec3::new(start_position.x, start_position.y + 1.0, start_position.z)
        } else {
            // Move in direction
            Vec3::new(start_position.x, start_position.y, start_position.z)
                + player_action_tracker.is_moving.value().as_vec3()
        };

        // Move player in direction
        commands.entity(player_entity).insert(PlayerMove {
            start_position,
            target_position,
        });
    }
}

fn process_player_action(
    time: Res<Time>,
    mut action_timer: ResMut<ActionTimer>,
    mut next_game_state: ResMut<NextState<GameState>>,
    player_action_tracker: Res<PlayerActionTracker>,
    mut player_query: Query<(Entity, &mut Transform, &PlayerMove), With<Player>>,
    mut commands: Commands,
) {
    action_timer.0.tick(time.delta());

    // Calculate progress (0.0 to 1.0) based on timer
    let progress = action_timer.0.fraction();

    // Smoothly interpolate player position based on timer progress
    if let Ok((_player_entity, mut player_transform, player_move)) = player_query.single_mut() {
        // Apply smooth easing function for more natural movement
        let eased_progress = ease_in_out_cubic(progress);

        // Lerp between start and target positions
        player_transform.translation = player_move
            .start_position
            .lerp(player_move.target_position, eased_progress);

        // For jump, add a slight arc
        if player_action_tracker.is_jumping {
            // Add a parabolic arc to the jump (highest at middle of animation)
            let jump_arc = 0.2 * (eased_progress * std::f32::consts::PI).sin();
            player_transform.translation.y += jump_arc;
        }
    }

    if action_timer.0.just_finished() {
        if let Ok((player_entity, mut player_transform, player_move)) = player_query.single_mut() {
            // Ensure player is exactly at target position
            player_transform.translation = player_move.target_position;

            // Remove the PlayerMove component
            commands.entity(player_entity).remove::<PlayerMove>();

            if player_action_tracker.is_jumping {
                info!("Player action (Jump start) animation finished. Player is now in mid-air.");
            } else {
                info!("Player action (Move) animation finished. Player moved forward.");
            }

            next_game_state.set(GameState::GameTurnInProgress);
        }
    }
}

fn update_player_pos_after_jump(
    mut game: ResMut<Game>,
    player_query: Query<&PlayerMove, With<Player>>,
) {
    if let Ok(player_move) = player_query.single() {
        game.player_pos = player_move.target_position.as_ivec3();
        debug!("Player position updated to: {:?}", game.player_pos);
    }
}

// Cubic easing function for smoother animation
fn ease_in_out_cubic(t: f32) -> f32 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

// Component for logs that are rolling
#[derive(Component)]
pub struct LogRoll {
    pub start_position: Vec3,  // Starting position of the log
    pub target_position: Vec3, // Target position after rolling
}

fn setup_game_turn_timer(
    mut action_timer: ResMut<ActionTimer>,
    mut commands: Commands,
    log_query: Query<(Entity, &Transform), With<Log>>,
) {
    info!(
        "GameState: GameTurnInProgress. Starting {GAME_MOVE_ANIMATION_DURATION}s game move timer."
    );
    action_timer.0 = Timer::from_seconds(GAME_MOVE_ANIMATION_DURATION, TimerMode::Once);
    action_timer.0.reset();

    // Add LogRoll component to all logs
    for (log_entity, log_transform) in log_query.iter() {
        let start_position = log_transform.translation;
        let target_position = Vec3::new(
            start_position.x,
            start_position.y,
            start_position.z - TILE_SIZE * 2.0,
        );
        commands.entity(log_entity).insert(LogRoll {
            start_position,
            target_position,
        });
    }
}

fn process_game_turn(
    time: Res<Time>,
    mut action_timer: ResMut<ActionTimer>,
    player_action_tracker: Res<PlayerActionTracker>,
    mut log_query: Query<(Entity, &mut Transform, &LogRoll), With<Log>>,
    game: Res<Game>,
    mut commands: Commands,
    mut next_game_state: ResMut<NextState<GameState>>,
) {
    action_timer.0.tick(time.delta());

    // Calculate progress (0.0 to 1.0) based on timer
    let progress = action_timer.0.fraction();

    // Smoothly interpolate log positions based on timer progress
    for (log_entity, mut log_transform, log_roll) in log_query.iter_mut() {
        // Lerp between start and target positions
        log_transform.translation = log_roll
            .start_position
            .lerp(log_roll.target_position, progress);

        // Check if log is out of bounds (only if we're close to completion)
        if progress > 0.9 {
            let board_pos = IVec3::new(
                log_transform.translation.x as i32,
                log_transform.translation.y as i32,
                log_transform.translation.z as i32,
            );

            if !game.is_valid_board_pos(board_pos, &Direction::West) {
                info!("Log went out of bounds! Despawning it.");
                commands.entity(log_entity).despawn();
            }
        }
    }

    if action_timer.0.just_finished() {
        // Make sure all logs are exactly at their target positions and reset rotation
        for (log_entity, mut log_transform, log_roll) in log_query.iter_mut() {
            log_transform.translation = log_roll.target_position;
            log_transform.rotation = Quat::IDENTITY; // Reset rotation to 0
            commands.entity(log_entity).remove::<LogRoll>(); // Remove the LogRoll component
        }

        info!("Game move animation finished. Log movement complete.");

        if player_action_tracker.is_jumping {
            info!("Player was jumping, proceeding to PlayerFinishingJump.");
            next_game_state.set(GameState::PlayerFinishingJump);
        } else {
            info!("Player was moving (not jumping), round ends. Back to PlayerIdle.");
            next_game_state.set(GameState::PlayerIdle);
        }
    }
}

// System to handle log rolling animation
fn roll_logs(time: Res<Time>, mut log_query: Query<&mut Transform, (With<Log>, With<LogRoll>)>) {
    for mut log_transform in log_query.iter_mut() {
        // Calculate roll speed based on distance
        let roll_speed = -PI; // Adjust as needed for rotation speed

        // Rotate around X axis (rolling forward/backward)
        let rotation = Quat::from_rotation_x(time.delta_secs() * roll_speed);
        log_transform.rotate(rotation);
    }
}

#[derive(Event)]
pub struct PlayerBirdRescueEvent;

fn increment_bevy(
    _trigger: Trigger<PlayerBirdRescueEvent>,
    mut game: ResMut<Game>,
    mut commands: Commands,
) {
    info!("birds left: {}", game.current_level().bird_map.len());
    game.bevy_count += 1;
    info!("bird count: {}", game.bevy_count);
    if game.current_level().bird_map.is_empty() {
        commands.trigger(GameEvent::Win);
    }
}

fn player_check_for_bird(
    player_query: Query<(Entity, &Transform), With<Player>>,
    mut bird_query: Query<&mut Transform, (With<Bird>, Without<Player>)>,
    mut commands: Commands,
    mut game: ResMut<Game>,
    player_action: Res<PlayerActionTracker>,
) {
    if player_action.is_jumping {
        info!("check for birds!");
        match player_query.single() {
            Ok((player_ent, tf)) => {
                let mut pos = tf.translation.as_ivec3();
                pos.y = BIRD_Y;
                if let Some(entity) = game.current_level_mut().bird_map.remove(&pos) {
                    commands.trigger(PlayerBirdRescueEvent);
                    if let Ok(mut bird_tf) = bird_query.get_mut(entity) {
                        let diff = (game.bevy_count + 1) as f32;
                        bird_tf.translation = Vec3::ZERO;
                        bird_tf.translation.y += 0.5 * diff;
                        bird_tf.rotate_y(0.65 * diff);
                    }
                    info!("bird found adding to bevy!");
                    commands.entity(player_ent).add_child(entity);
                }
            }
            _ => (),
        }
    }
}

fn setup_player_jump_land_timer(
    mut action_timer: ResMut<ActionTimer>,
    mut commands: Commands,
    player_query: Query<(Entity, &Transform), With<Player>>,
) {
    info!(
        "GameState: PlayerFinishingJump. Starting {PLAYER_JUMP_LAND_ANIMATION_DURATION}s jump landing timer."
    );
    action_timer.0 = Timer::from_seconds(PLAYER_JUMP_LAND_ANIMATION_DURATION, TimerMode::Once);
    action_timer.0.reset();

    // Set up player landing animation
    if let Ok((player_entity, player_transform)) = player_query.single() {
        let start_position = player_transform.translation;
        let target_position = Vec3::new(start_position.x, start_position.y - 1.0, start_position.z);

        commands.entity(player_entity).insert(PlayerMove {
            start_position,
            target_position,
        });
    }
}

fn process_player_finishing_jump(
    time: Res<Time>,
    mut action_timer: ResMut<ActionTimer>,
    mut next_game_state: ResMut<NextState<GameState>>,
    mut player_action_tracker: ResMut<PlayerActionTracker>,
    mut player_query: Query<(Entity, &mut Transform, &PlayerMove), With<Player>>,
    mut commands: Commands,
) {
    action_timer.0.tick(time.delta());
    let progress = action_timer.0.fraction();

    // Smoothly interpolate player position based on timer progress
    if let Ok((_player_entity, mut player_transform, player_move)) = player_query.single_mut() {
        // Use different easing for landing (more impactful)
        let eased_progress = easeoutbounce(progress).abs();

        player_transform.translation = player_move
            .start_position
            .lerp(player_move.target_position, eased_progress);
    }

    if action_timer.0.just_finished() {
        if let Ok((player_entity, mut player_transform, player_move)) = player_query.single_mut() {
            // Ensure player is exactly at target position, this should really check it is very close
            player_transform.translation = player_move.target_position;

            commands.entity(player_entity).remove::<PlayerMove>();

            info!(
                "Player jump landing animation finished. Jump complete and player returned to ground."
            );
            player_action_tracker.is_jumping = false; // Reset for the next turn
            next_game_state.set(GameState::PlayerIdle);
        }
    }
}

fn easeoutbounce(mut t: f32) -> f32 {
    let n1 = 7.5625;
    let d1 = 2.75;

    if t < 1.0 / d1 {
        return n1 * t * t;
    } else if t < 2.0 / d1 {
        t -= 1.5 / d1;
        return n1 * t * t + 0.75;
    } else if t < 2.5 / d1 {
        t -= 2.25 / d1;
        return n1 * t * t + -0.9375;
    } else {
        t -= 2.625 / d1;
        return n1 * t * t + 0.984375;
    }
}

// Resource to store the GameState before entering Paused
#[derive(Resource, Default)]
struct PrevState(Option<GameState>);

fn toggle_pause(
    input: Res<ButtonInput<KeyCode>>,
    current_game_state: Res<State<GameState>>,
    mut next_game_state: ResMut<NextState<GameState>>,
    mut prev_state: ResMut<PrevState>, // Add PrevState resource
) {
    if input.just_pressed(KeyCode::Space) {
        match current_game_state.get() {
            GameState::Paused => {
                info!("Resuming game.");
                // Retrieve the state before pausing
                if let Some(previous_state) = prev_state.0.take() {
                    // Use take() to clear the resource
                    info!("Resuming to {:?}", previous_state);
                    next_game_state.set(previous_state);
                } else {
                    warn!(
                        "Attempted to resume from Paused, but no previous state was recorded. Resuming to PlayerIdle."
                    );
                    next_game_state.set(GameState::PlayerIdle);
                }
            }
            // Handle the case where the current state is *not* Paused
            other_state => {
                info!("Pausing game from {:?}.", other_state);
                // Store the current state before transitioning to Paused
                prev_state.0 = Some(*other_state);
                next_game_state.set(GameState::Paused);
            }
        }
    }
}

fn setup_paused_screen() {
    info!("GameState: Paused. Game is paused. Press Space to resume.");
    // Here you would spawn a pause menu UI
}

fn cleanup_paused_screen() {
    info!("Exiting GameState: Paused. Resuming game activities.");
    // Here you would despawn the pause menu UI
}

fn log_gamestate_transitions(mut transitions: EventReader<StateTransitionEvent<GameState>>) {
    for transition in transitions.read() {
        info!(
            "GAMESTATE TRANSITION: {:?} => {:?}",
            transition.entered, transition.exited
        );
    }
}

#[derive(Component)]
struct Rotate {
    speed: f32, // Rotation speed in radians per second
}

impl Default for Rotate {
    fn default() -> Self {
        Self { speed: 1.0 }
    }
}

fn rotate_system(time: Res<Time>, mut query: Query<(&Rotate, &mut Transform)>) {
    for (rotate, mut transform) in query.iter_mut() {
        transform.rotate_y(rotate.speed * time.delta_secs());
    }
}

#[derive(Resource, Default)]
struct DebugSkipPlayerAction {
    pub skip_player_action: bool,
    pub skip_player_collision: bool,
}

// Add this system definition near the other systems
fn toggle_debug_skip_player_action(
    input: Res<ButtonInput<KeyCode>>,
    mut debug_skip_player_action: ResMut<DebugSkipPlayerAction>,
) {
    if input.just_pressed(KeyCode::KeyZ) {
        debug_skip_player_action.skip_player_action = !debug_skip_player_action.skip_player_action;
        info!(
            "Debug skip player action toggled: {}",
            debug_skip_player_action.skip_player_action
        );
    }
    if input.just_pressed(KeyCode::KeyX) {
        debug_skip_player_action.skip_player_collision =
            !debug_skip_player_action.skip_player_collision;
        info!(
            "Debug skip player collision toggled: {}",
            debug_skip_player_action.skip_player_collision
        );
    }
}

fn text_update_bird_count(mut query: Query<&mut TextSpan, With<BirdCountText>>, game: Res<Game>) {
    for mut span in &mut query {
        **span = format!("{}", game.bevy_count);
    }
}

fn text_update_game_message(
    trigger: Trigger<GameEvent>,
    mut query: Query<(&mut Text, &mut Visibility), With<GameMessage>>,
) {
    for (mut text, mut vis) in &mut query {
        *vis = Visibility::Visible;
        **text = format!("{}", trigger.event().text());
    }
}

fn text_update_game_message_hide(mut query: Query<&mut Visibility, With<GameMessage>>) {
    for mut vis in &mut query {
        *vis = Visibility::Hidden;
    }
}
