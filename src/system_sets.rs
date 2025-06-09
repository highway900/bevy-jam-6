use bevy::prelude::*;

use crate::{
    camera::{spawn_camera, switch_projection, zoom},
    debug::render_debug_gizmos,
    game_logic::{game_step_event_handle, player_state_next_event_handle, update_game_loop},
    init::setup,
    input::{exit_on_esc, handle_keys},
    loading::game_added,
    logs::{init_logs, log_action, log_roll, logs_move_finished, spawn_logs},
    player::*,
};

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum SystemSets {
    DebugSet,
    LoadSetStartup,
    LoadSetUpdate,
    InputSet,
    GameSetStartup,
    GameSetUpdate,
    PlayerSetUpdate,
}

impl SystemSets {
    pub fn systems(
        &self,
    ) -> bevy::ecs::schedule::ScheduleConfigs<
        Box<dyn System<In = (), Out = std::result::Result<(), BevyError>>>,
    > {
        match self {
            SystemSets::DebugSet => (render_debug_gizmos).in_set(self.clone()),
            SystemSets::LoadSetStartup => (setup, spawn_camera).chain().in_set(self.clone()),
            SystemSets::LoadSetUpdate => (game_added).chain().in_set(self.clone()),
            SystemSets::GameSetStartup => (spawn_player, init_logs).in_set(self.clone()),
            SystemSets::GameSetUpdate => (
                update_game_loop,
                player_cursor_update,
                game_step_event_handle,
                spawn_logs,
                logs_move_finished,
                log_action,
                log_roll,
            )
                .chain()
                .in_set(self.clone()),
            SystemSets::InputSet => {
                (handle_keys, exit_on_esc, switch_projection, zoom).in_set(self.clone())
            }
            SystemSets::PlayerSetUpdate => (
                player_state_next_event_handle,
                player_act,
                player_jump,
                // player_move,
                // player_move_complete,
            )
                .chain()
                .in_set(self.clone()),
        }
    }
}
