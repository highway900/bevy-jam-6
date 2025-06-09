use std::{collections::VecDeque, ops::Range};

use bevy::prelude::*;

use bevy_inspector_egui::prelude::*;

use crate::player::{Direction, PlayerAction};

#[derive(Resource, Clone, Copy, Reflect, InspectorOptions)]
#[reflect(Resource, InspectorOptions)]
pub struct Configuration {
    pub show_aabbs: bool,
}

impl Default for Configuration {
    fn default() -> Self {
        Self { show_aabbs: false }
    }
}

#[derive(Clone, Reflect, Default, Eq, PartialEq, Debug)]
pub enum GameState {
    #[default]
    None,
    Playing,
    PlayerAction,
    GameAction,
    GameActionComplete,
}

#[derive(Resource, Clone, Reflect, InspectorOptions)]
#[reflect(Resource, InspectorOptions)]
pub struct Game {
    pub board_size_x: u32,
    pub board_size_y: u32,
    pub start: IVec2,
    pub player_pos: IVec3,
    pub current_step: u32,
    pub action_list: Vec<PlayerAction>,
    pub player_action_next: PlayerAction,
    pub state: GameState,
    pub log_sequence: VecDeque<u32>,
    pub logs: Vec<Entity>,
}

impl Default for Game {
    fn default() -> Self {
        Self {
            board_size_x: 12,
            board_size_y: 12,
            start: IVec2::new(3, 0),
            current_step: 0,
            action_list: vec![],
            player_action_next: PlayerAction::default(),
            state: GameState::default(),
            player_pos: IVec3::new(3, 0, 0),
            log_sequence: VecDeque::from([3]),
            logs: Vec::with_capacity(24),
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
        if pos.x > -1 // XXX: Weird off by 2...
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
}

#[allow(dead_code)]
#[derive(Resource)]
pub struct Boundary {
    pub xy_min: Vec2,
    pub xy_max: Vec2,
}

#[derive(Resource, Clone, Reflect, InspectorOptions)]
#[reflect(Resource, InspectorOptions)]
pub struct CameraSettings {
    /// The height of the viewport in world units when the orthographic camera's scale is 1
    pub orthographic_viewport_height: f32,
    /// Clamp the orthographic camera's scale to this range
    pub orthographic_zoom_range: Range<f32>,
    /// Multiply mouse wheel inputs by this factor when using the orthographic camera
    pub orthographic_zoom_speed: f32,
    /// Clamp perspective camera's field of view to this range
    pub perspective_zoom_range: Range<f32>,
    /// Multiply mouse wheel inputs by this factor when using the perspective camera
    pub perspective_zoom_speed: f32,
}
