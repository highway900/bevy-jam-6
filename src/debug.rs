use bevy::{
    color::palettes::css::{GREEN, PURPLE},
    prelude::*,
};

use crate::{Configuration, TILE_SIZE, resources::Game};

/// Used in systemsets to toggle running systems
pub fn is_debug_enabled(config: Res<Configuration>) -> bool {
    config.show_aabbs
}

pub fn render_debug_gizmos(mut gizmos: Gizmos, game: Res<Game>) {
    gizmos.axes(Transform::from_translation(Vec3::ZERO), 0.5);
    let ps = game.player_pos.as_vec3();
    gizmos.cross(Isometry3d::from_translation(ps), TILE_SIZE, PURPLE);

    for y in 0..game.board_size_y {
        for x in 0..game.board_size_x {
            gizmos.cross(
                Isometry3d::from_translation(Vec3::new(x as f32, 0.1, y as f32)),
                TILE_SIZE,
                GREEN,
            );
        }
    }
}
