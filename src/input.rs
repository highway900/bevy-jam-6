use bevy::prelude::*;

use crate::{
    events::GameEvents,
    player::{Direction, PlayerAction, PlayerState},
};

pub fn exit_on_esc(kb_input: Res<ButtonInput<KeyCode>>) {
    if kb_input.just_released(KeyCode::Escape) {
        std::process::exit(0)
    }
}

pub fn handle_keys(
    kb: Res<ButtonInput<KeyCode>>,
    mut game_events: EventWriter<GameEvents>,
    mut player_events: EventWriter<PlayerState>,
) {
    if kb.just_released(KeyCode::KeyA) || kb.just_released(KeyCode::ArrowLeft) {
        let id = player_events.write(PlayerState::Next(PlayerAction::Move(Direction::East)));
        info!("Key Player Move released: {id:?}");
    }
    if kb.just_released(KeyCode::KeyD) || kb.just_released(KeyCode::ArrowRight) {
        let id = player_events.write(PlayerState::Next(PlayerAction::Move(Direction::West)));
        info!("Key Player Move released: {id:?}");
    }
    if kb.just_released(KeyCode::KeyW) || kb.just_released(KeyCode::ArrowUp) {
        let id = player_events.write(PlayerState::Next(PlayerAction::Move(Direction::North)));
        info!("Key Player Move released: {id:?}");
    }
    if kb.just_released(KeyCode::KeyS) || kb.just_released(KeyCode::ArrowDown) {
        let id = player_events.write(PlayerState::Next(PlayerAction::Move(Direction::South)));
        info!("Key Player Move released: {id:?}");
    }
    if kb.just_released(KeyCode::Space) {
        let id = game_events.write(GameEvents::StepForward);
        info!("Key Advance released: {id:?}");
    }
    if kb.just_released(KeyCode::KeyX) {
        info!("Key X released");
    }
}
