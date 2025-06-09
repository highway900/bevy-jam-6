use bevy::{color::palettes::css::WHITE, prelude::*};

use crate::{
    CURSOR_Y, TILE_SIZE,
    resources::{Game, GameState},
};

#[derive(Component)]
pub struct Player;

#[derive(Clone, Reflect, Debug, PartialEq)]
pub enum Direction {
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
        }
    }
}

#[derive(Component, Event, Clone, Reflect, Debug, PartialEq)]
pub enum PlayerAction {
    Move(Direction),
    Jump(JumpState),
    CartWheel(Direction),
    Complete,
}

impl Default for PlayerAction {
    fn default() -> Self {
        PlayerAction::Jump(JumpState::default())
    }
}

#[derive(Component, Event, Clone, Reflect, Debug, Default, PartialEq)]
pub enum JumpState {
    #[default]
    Start,
    MidAir,
    Complete,
}

#[derive(Event, Debug)]
pub enum PlayerState {
    Next(PlayerAction),
    Prev(PlayerAction),
}

#[derive(Component)]
pub struct PlayerCursor;

#[derive(Component)]
pub struct PlayerTimer(Timer);

pub fn spawn_player(
    mut commands: Commands,
    game: Res<Game>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    info!("spawning player");
    commands.spawn((
        Transform::from_xyz(game.start.x as f32, CURSOR_Y, game.start.y as f32),
        PlayerCursor,
        Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(TILE_SIZE)))),
        MeshMaterial3d(materials.add(Color::linear_rgba(1.0, 1.0, 1.0, 0.7))),
        ShowAabbGizmo {
            color: Some(Color::from(WHITE)),
        },
    ));
    commands
        .spawn((
            Transform::from_xyz(game.start.x as f32, TILE_SIZE, game.start.y as f32),
            Player,
            Mesh3d(meshes.add(Cuboid::new(TILE_SIZE, TILE_SIZE * 2.2, TILE_SIZE))),
            MeshMaterial3d(materials.add(Color::linear_rgb(0.8, 0.1, 0.1))),
        ))
        .with_children(|parent| {
            // Nose (direction)
            parent.spawn((
                Transform::from_xyz(0.0, TILE_SIZE - 0.1, TILE_SIZE),
                Mesh3d(meshes.add(Cuboid::new(0.05, 0.05, 0.2))),
                MeshMaterial3d(materials.add(Color::linear_rgb(1., 0.875, 0.545))),
            ));
        });
}

pub fn player_cursor_update(
    mut cursor: Query<&mut Transform, With<PlayerCursor>>,
    mut player_events: EventReader<PlayerState>,
    game: Res<Game>,
) {
    for ev in player_events.read() {
        let Ok(mut cursor) = cursor.single_mut() else {
            return;
        };
        match ev {
            PlayerState::Next(player_action) => match player_action {
                PlayerAction::Move(direction) => {
                    if game.is_valid_player_move(direction) {
                        let next_pos = game.player_pos + direction.value();
                        info!("cursor valid move: {:?}", next_pos);
                        cursor.translation = next_pos.as_vec3();
                        cursor.translation.y = 0.01;
                    }
                }
                _ => (),
            },
            PlayerState::Prev(_player_action) => todo!(),
        }
    }
}

pub fn player_act(
    mut player_events: EventReader<PlayerAction>,
    mut game: ResMut<Game>,
    player: Query<(Entity, Option<&PlayerTimer>), With<Player>>,
    mut commands: Commands,
) {
    for player_action in player_events.read() {
        match player_action {
            PlayerAction::Move(direction) => {
                if game.is_valid_player_move(direction) {
                    info!("{player_action:?}");
                }
            }
            PlayerAction::Jump(jump_state) => {
                let Ok((entity, existing_timer)) = player.single() else {
                    return;
                };
                match jump_state {
                    JumpState::Start => {
                        info!("{player_action:?}");
                        commands.entity(entity).insert((
                            PlayerAction::Jump(jump_state.clone()),
                            PlayerTimer(Timer::from_seconds(0.18, TimerMode::Once)),
                        ));
                    }
                    JumpState::MidAir => {
                        info!("{player_action:?}");
                        game.player_action_next = PlayerAction::Jump(JumpState::Complete);
                        game.state = GameState::GameAction;
                    }
                    JumpState::Complete => {
                        info!(">> {player_action:?}");
                        // Only insert timer if it doesn't already exist
                        if existing_timer.is_none() {
                            info!("Inserting new complete timer");
                            commands.entity(entity).insert((
                                PlayerAction::Jump(jump_state.clone()),
                                PlayerTimer(Timer::from_seconds(0.14, TimerMode::Once)),
                            ));
                        } else {
                            info!("Timer already exists, not reinserting");
                        }
                    }
                }
            }
            PlayerAction::CartWheel(_direction) => todo!(),
            PlayerAction::Complete => {
                info!("++ {player_action:?}");
                game.state = GameState::None;
            }
        }
    }
}

pub fn player_jump(
    mut commands: Commands,
    time: Res<Time>,
    mut player_query: Query<
        (Entity, &mut Transform, &mut PlayerTimer, &mut PlayerAction),
        (With<Player>, With<PlayerAction>), // Query for Player with an active Action and Timer
    >,
    mut player_action_events: EventWriter<PlayerAction>,
) {
    for (entity, mut tform, mut timer, mut action) in player_query.iter_mut() {
        let current_jump_state = match &mut *action {
            PlayerAction::Jump(state) => state,
            _ => continue,
        };
        info!("player_jump: {:?}", current_jump_state);
        match current_jump_state {
            JumpState::Start => {
                if timer.0.tick(time.delta()).finished() {
                    info!("{action:?}");
                    tform.translation.y += 1.0;
                    commands.entity(entity).remove::<PlayerAction>();
                    commands
                        .entity(entity)
                        .insert(PlayerAction::Jump(JumpState::MidAir));
                    player_action_events.write(PlayerAction::Jump(JumpState::MidAir));
                }
            }
            JumpState::MidAir => {
                info!("{action:?}")
            }
            JumpState::Complete => {
                timer.0.tick(time.delta());
                info!(
                    "^^^^ {action:?} - {:?} - {:?}",
                    (timer.0.elapsed(), timer.0.finished()),
                    time.delta()
                );
                if timer.0.finished() {
                    info!("<<(timer finished) {action:?}");
                    tform.translation.y -= 1.0;
                    commands
                        .entity(entity)
                        .remove::<(PlayerAction, PlayerTimer)>();
                    player_action_events.write(PlayerAction::Complete);
                };
            }
        }
    }
}

pub fn player_move(
    time: Res<Time>,
    mut player: Query<(&mut Transform, &mut PlayerTimer, &PlayerAction), With<Player>>,
) {
    for (mut tform, timer, action) in player.iter_mut() {
        if let PlayerAction::Move(direction) = action {
            // Calculate the amount to move this frame based on the timer duration
            let move_displacement_this_frame = direction.value().as_vec3()
                * (time.delta_secs() / timer.0.duration().as_secs_f32());
            // Apply the movement
            tform.translation += move_displacement_this_frame;
        }
    }
}

pub fn player_move_complete(
    mut commands: Commands,
    time: Res<Time>,
    mut player: Query<(Entity, &mut Transform, &mut PlayerTimer, &PlayerAction), With<Player>>,
    mut player_action_events: EventWriter<PlayerAction>,
) {
    for (entity, mut tform, mut timer, action) in player.iter_mut() {
        if let PlayerAction::Move(_direction) = action {
            if timer.0.tick(time.delta()).finished() {
                info!("timer finished!");
                tform.translation.y = TILE_SIZE;
                commands
                    .entity(entity)
                    .remove::<(PlayerTimer, PlayerAction)>();
                player_action_events.write(PlayerAction::Complete);
            }
        }
    }
}
