use crate::{
    TILE_COLOR, TILE_SIZE,
    components::Tile,
    resources::{Boundary, Game},
    states::AppState,
};
use bevy::gizmos::aabb::ShowAabbGizmo;
use bevy::prelude::*;

pub fn game_added(
    mut commands: Commands,
    game: Res<Game>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut next_app_state: ResMut<NextState<AppState>>,
) {
    if game.is_added() {
        commands.insert_resource(Boundary {
            xy_min: Vec2::splat(0.0),
            xy_max: Vec2::new(game.board_size_x as f32, game.board_size_y as f32),
        });

        for y in 0..game.board_size_y {
            for x in 0..game.board_size_x {
                let entity = commands
                    .spawn((Tile, Transform::from_xyz(x as f32, 0.0, y as f32)))
                    .id();
                commands.entity(entity).insert((
                    Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(TILE_SIZE)))),
                    MeshMaterial3d(materials.add(TILE_COLOR)),
                    // PIXEL_PERFECT_LAYERS,
                    ShowAabbGizmo {
                        color: Some(Color::linear_rgba(0.51, 0.34, 0.075, 0.75)),
                    },
                ));
            }
        }

        info!("Next state InGame");
        next_app_state.set(AppState::InGame);
    }
}
