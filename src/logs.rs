use bevy::prelude::*;

use crate::{
    TILE_SIZE,
    events::GameEvents,
    player::Direction,
    resources::{Game, GameState},
};

#[derive(Event, Default)]
pub enum LogAction {
    #[default]
    Idle,
    Moving,
    Spawn,
}

#[derive(Component)]
pub struct LogLog;

#[derive(Component)]
pub struct LogLogDest(pub Vec3);

#[derive(Component)]
pub struct LogLogArrived;

#[derive(Component)]
pub enum LogStyle {
    Simple,
}

impl LogStyle {
    pub fn movement(&self) -> IVec3 {
        match self {
            LogStyle::Simple => IVec3::new(0, 0, -2),
        }
    }
}

#[derive(Component)]
pub struct LogSpawner;

fn make_dest_style(pos: Vec3, log_style: LogStyle) -> LogLogDest {
    let pos = pos + log_style.movement().as_vec3();
    LogLogDest(pos)
}

pub fn init_logs(mut log_events: EventWriter<LogAction>) {
    log_events.write(LogAction::Spawn);
}

pub fn spawn_logs(
    mut game: ResMut<Game>,
    mut events: EventReader<GameEvents>,
    mut log_events: EventWriter<LogAction>,
) {
    for ev in events.read() {
        match ev {
            GameEvents::StepForward => {
                info!(
                    "step forward: spawn logs - step: {:?}, seq: {:?}",
                    game.current_step, game.log_sequence
                );
                if game.log_sequence.front() == Some(&game.current_step) {
                    game.log_sequence.pop_front();
                    info!(
                        "spawning new log: {:?}, {:?}",
                        game.current_step, game.log_sequence
                    );
                    log_events.write(LogAction::Spawn);
                }
            }
            _ => (),
        }
    }
}

pub fn logs_move_finished(
    logs: Query<Entity, With<LogLog>>,
    logs_dest: Query<&LogLogArrived>,
    mut game: ResMut<Game>,
    mut commands: Commands,
) {
    if logs.is_empty() && game.state == GameState::Playing {
        info!("No logs present, continuing to next game state");
        game.state = GameState::GameActionComplete;
        return;
    }

    if logs_dest.iter().count() == logs.iter().count()
        && !logs.is_empty()
        && game.state == GameState::Playing
    {
        info!("log has finished rolling");
        game.state = GameState::GameActionComplete;
        logs.iter().for_each(|entity| {
            commands
                .entity(entity)
                .remove::<(LogLogArrived, LogLogDest)>();
        });
    }
}

pub fn log_action(
    mut commands: Commands,
    query: Query<(Entity, &Transform), With<LogLog>>,
    mut log_action_event: EventReader<LogAction>,
    mut game: ResMut<Game>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for ev in log_action_event.read() {
        match ev {
            LogAction::Idle => {}
            LogAction::Moving => {
                if query.is_empty() && game.state == GameState::Playing {
                    info!(">> no logs skipping");
                    game.state = GameState::GameActionComplete;
                    return;
                }
                info!("LogAction::Moving");
                for (entity, tform) in query.iter() {
                    commands
                        .entity(entity)
                        .insert(make_dest_style(tform.translation, LogStyle::Simple));
                }
            }
            LogAction::Spawn => {
                let board = game.board_size_as_vec3();
                let pos = Transform::from_translation(Vec3::new(
                    4.0,
                    TILE_SIZE / 2.,
                    board.z - TILE_SIZE * 2.0,
                ));
                commands.spawn((
                    LogLog,
                    pos.clone(),
                    Mesh3d(meshes.add(Cuboid::new(TILE_SIZE * 6.0, TILE_SIZE, TILE_SIZE))),
                    MeshMaterial3d(materials.add(Color::linear_rgb(0.8, 0.1, 0.1))),
                ));
            }
        }
    }
}

pub fn log_roll(
    mut query: Query<(Entity, &mut Transform, &LogLogDest), (With<LogLog>, With<LogLogDest>)>,
    mut commands: Commands,
    time: Res<Time>,
    game: Res<Game>,
) {
    let speed = 1.25; // Units per second, adjust as needed
    for (entity, mut transform, dest) in query.iter_mut() {
        let target_position = dest.0;
        let current_position = transform.translation;

        let direction = target_position - current_position;
        let distance_squared = direction.length_squared();

        // Use a small epsilon to check if we are effectively at the destination
        if distance_squared < 1e-6 {
            info!("log arrived: {:?}", transform.translation.as_ivec3());
            commands.entity(entity).remove::<LogLogDest>();
            commands.entity(entity).insert(LogLogArrived);
            continue; // Move to the next log
        }

        // Calculate potential movement step for this frame
        let movement_step = direction.normalize() * speed * time.delta_secs();
        let movement_step_distance_squared = movement_step.length_squared();

        let actual_movement: Vec3;

        // Check if the movement step is larger than or equal to the remaining distance
        if movement_step_distance_squared >= distance_squared {
            // Snap to the destination
            actual_movement = direction; // The movement vector that takes us exactly there
            transform.translation = target_position;
        } else {
            // Move incrementally
            actual_movement = movement_step;
            transform.translation += actual_movement;
        }

        if !game.is_valid_board_pos(transform.translation.as_ivec3(), &Direction::South) {
            info!(
                "log finished: {:?} - {:?}",
                transform.translation.as_ivec3(),
                game.state
            );
            commands.entity(entity).despawn(); // Despawn when off the edge
            continue;
        }

        // Calculate rotation based on Z movement: 1 unit Z movement = 1 full rotation (2*PI radians)
        // Assuming rolling around the X axis for forward/backward movement
        // Positive Z movement means rolling 'forward', which typically corresponds
        // to a negative rotation around the X axis in a right-handed coordinate system.
        let rotation_angle = actual_movement.z * std::f32::consts::PI; // TAU = 2 * PI
        let rotation_quat = Quat::from_rotation_x(rotation_angle);

        // Apply the calculated rotation to the existing rotation
        transform.rotation *= rotation_quat;
    }
}
