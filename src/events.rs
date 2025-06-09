use bevy::prelude::*;

#[derive(Event)]
pub enum GameEvents {
    SpawnLogs,
    StepForward,
    StepBackward,
    StepComplete,
}
