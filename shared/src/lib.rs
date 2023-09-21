use std::collections::HashMap;

use serde::{Serialize, Deserialize};

pub const TICKRATE: u128 = 64;
/// measured in 10* millisecond.
/// eg if TIME_PER_TICK == 156 => 15.6 milliseconds
pub const TIME_PER_TICK: u128 = 10_000 / TICKRATE;
pub const MAX_MOVEMENT: f32 = 20.0;

pub const START_X_Y: f32 = 50.0;
pub const MIN_X_Y: f32 = 0.0;
pub const MAX_X_Y: f32 = 500.0;

pub fn fix_position_within_bounds(x: &mut f32, y: &mut f32) {
    if *x < MIN_X_Y {
        *x = MIN_X_Y;
    }
    if *x > MAX_X_Y {
        *x = MAX_X_Y
    }
    if *y < MIN_X_Y {
        *y = MIN_X_Y;
    }
    if *y > MAX_X_Y {
        *y = MAX_X_Y;
    }
}

#[derive(Serialize, Deserialize)]
pub enum GameOutputMessage {
    PlayerPositions { positions: HashMap<u64, (f32, f32)> },
    YouAre { id: u64 },
}

impl GameOutputMessage {
    pub fn serialize_json(&self) -> String {
        serde_json::to_string(self).expect("serialization failed")
    }
    pub fn deserialize_json(s: &str) -> Self {
        serde_json::from_str(s).expect(&format!("Failed to deserialize json string {s}"))
    }
}

#[derive(Deserialize, Serialize)]
pub enum GameInputMessage {
    Move { mx: f32, my: f32 }
}

impl GameInputMessage {
    pub fn serialize_json(&self) -> String {
        serde_json::to_string(self).expect("serialization failed")
    }
    pub fn deserialize_json(s: &str) -> Self {
        serde_json::from_str(s).expect(&format!("Failed to deserialize json string {s}"))
    }
}
