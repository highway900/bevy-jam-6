use std::f32::consts::PI;

use bevy::prelude::*;

use crate::{
    LIGHT_DIR_YELLOW_WHITE,
    resources::{CameraSettings, Game},
};

pub fn setup(mut commands: Commands) {
    warn!("TODO!: load from RON file");
    commands.insert_resource(Game::default());
    commands.insert_resource(CameraSettings {
        orthographic_viewport_height: 5.,
        // In orthographic projections, we specify camera scale relative to a default value of 1,
        // in which one unit in world space corresponds to one pixel.
        orthographic_zoom_range: 0.1..10.0,
        // This value was hand-tuned to ensure that zooming in and out feels smooth but not slow.
        orthographic_zoom_speed: 0.2,
        // Perspective projections use field of view, expressed in radians. We would
        // normally not set it to more than π, which represents a 180° FOV.
        perspective_zoom_range: (PI / 5.)..(PI - 0.2),
        // Changes in FOV are much more noticeable due to its limited range in radians
        perspective_zoom_speed: 0.05,
    });

    commands.insert_resource(AmbientLight {
        brightness: 80.0,
        color: Color::WHITE,
        ..Default::default()
    });

    let light_position = Vec3::new(3.5, 5.0, 3.5);
    let light_intensity_watts = 1000.0;
    commands.spawn((
        Name::new("light_sun"),
        DirectionalLight {
            illuminance: light_intensity_watts, // Adjust as needed
            shadows_enabled: true,              // Enable shadows for better depth perception
            color: Color::from(LIGHT_DIR_YELLOW_WHITE),
            ..default()
        },
        Transform::from_translation(light_position).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
