use bevy::prelude::*;

use crate::{
    events::GameEvents,
    logs::LogAction,
    player::{PlayerAction, PlayerState},
    resources::{Game, GameState},
};

pub fn player_state_next_event_handle(
    mut player_events: EventReader<PlayerState>,
    mut game: ResMut<Game>,
) {
    // Don't allow updates when the game is in play
    if game.state == GameState::Playing {
        return;
    }
    for ev in player_events.read() {
        match ev {
            PlayerState::Next(player_action) => {
                info!("update player action: {ev:?} -> {player_action:?}");
                game.player_action_next = player_action.clone();
            }
            PlayerState::Prev(_player_action) => info!("update player action: {ev:?}"),
        }
    }
}

pub fn game_step_event_handle(
    mut events: EventReader<GameEvents>,
    mut game: ResMut<Game>,
    mut player_events: EventWriter<PlayerState>,
) {
    for ev in events.read() {
        match ev {
            GameEvents::StepForward => {
                if game.state == GameState::None {
                    game.current_step = game.current_step + 1;
                    game.state = GameState::PlayerAction;
                    info!("Step Forward: {:?}", game.current_step);
                }
            }
            GameEvents::StepBackward => {
                info!("Step Backward: {:?}", game.current_step);
            }
            GameEvents::StepComplete => {
                info!("step complete: step {:?}", game.current_step);
                info!("player action: {:?}", game.player_action_next);
                player_events.write(PlayerState::Next(PlayerAction::default()));
            }
            _ => (),
        }
    }
}

pub fn update_game_loop(
    mut game: ResMut<Game>,
    mut player_action_event: EventWriter<PlayerAction>,
    mut game_action_event: EventWriter<LogAction>,
    // mut game_events: EventWriter<GameEvents>,
) {
    match game.state {
        GameState::None => {
            game_action_event.write(LogAction::Idle);
            game.player_action_next = PlayerAction::default();
        }
        GameState::Playing => {}
        GameState::PlayerAction => {
            info!("player action, step: {:?}", game.current_step);
            game.state = GameState::Playing;
            player_action_event.write(game.player_action_next.clone());
        }
        GameState::GameAction => {
            info!("game action, step: {:?}", game.current_step);
            game.state = GameState::Playing;
            game_action_event.write(LogAction::Moving);
        }
        GameState::GameActionComplete => {
            info!("game action complete, step: {:?}", game.current_step);
            game.state = GameState::Playing;
            game_action_event.write(LogAction::Idle);
            info!("next action player: {:?}", game.player_action_next);
            player_action_event.write(game.player_action_next.clone());
        }
    }
}
