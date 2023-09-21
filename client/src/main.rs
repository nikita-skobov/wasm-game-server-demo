use macroquad::prelude::*;
use sapp_jsutils::JsObject;
use shared::*;
use std::{sync::{OnceLock, Mutex}, collections::HashMap};

fn ws_messages() -> &'static mut Mutex<Vec<String>> {
    static mut ARRAY: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
    unsafe {
        ARRAY.get_or_init(|| Mutex::new(vec![]));
        ARRAY.get_mut().unwrap()
    }
}

extern "C" {
    fn send_ws_message(jsobj: JsObject);
}

fn send_message(msg: GameInputMessage) {
    unsafe {
        let s = msg.serialize_json();
        let obj = JsObject::string(&s);
        send_ws_message(obj);
    }
}

#[no_mangle]
pub extern "C" fn push_ws_message(jsobj: JsObject) {
    let mut message = String::new();
    jsobj.to_string(&mut message);
    if let Ok(mut l) = ws_messages().lock() {
        l.push(message);
    } else {
        miniquad::error!("Failed to get ws lock1!");
    }
}

pub fn handle_server_events(state: &mut ClientState) {
    let data = match ws_messages().lock() {
        Ok(mut items) => {
            std::mem::take::<Vec<String>>(&mut items.as_mut())
        }
        Err(_) => {
            miniquad::error!("Failed to get ws lock2!");
            return
        },
    };
    for ev in data {
        let ev = GameOutputMessage::deserialize_json(&ev);
        match ev {
            GameOutputMessage::PlayerPositions { positions } => {
                for (id, pos) in positions {
                    miniquad::warn!("Player {} is at {:?}", id, pos);
                    let is_my_id = if let Some(my_id) = state.connected_id {
                        my_id == id
                    } else {
                        false
                    };
                    if is_my_id { continue; }
                    match state.other_players.get_mut(&id) {
                        Some(existing) => {
                            existing.1 = pos;
                        }
                        None => {
                            let mut rng = fastrand::Rng::with_seed(id);
                            let h = rng.f32();
                            let color = macroquad::color::hsl_to_rgb(h, 1.0, 0.5);
                            state.other_players.insert(id, (color, pos));
                        }
                    }
                }
            }
            GameOutputMessage::YouAre { id } => {
                let mut rng = fastrand::Rng::with_seed(id);
                let h = rng.f32();
                let color = macroquad::color::hsl_to_rgb(h, 1.0, 0.5);
                state.connected_id = Some(id);
                state.color = color;
            }
        }
    }
}

#[derive(Default)]
pub struct ClientState {
    pub other_players: HashMap<u64, (Color, (f32, f32))>,
    pub connected_id: Option<u64>,
    pub x: f32,
    pub y: f32,
    pub color: Color,
}

#[macroquad::main("BasicShapes")]
async fn main() {
    let mut state = ClientState::default();
    state.x = START_X_Y;
    state.y = START_X_Y;
    loop {
        let (my_id, b_color) = if let Some(id) = state.connected_id {
            (id, BLACK)
        } else {
            (0, GRAY)
        };
        clear_background(b_color);
        handle_server_events(&mut state);
        if state.connected_id.is_none() {
            draw_text("CONNECTING...", 20.0, 20.0, 48.0, WHITE);
            next_frame().await;
            continue;
        }
        let mut dx = 0.0;
        let mut dy = 0.0;
        if is_key_down(KeyCode::A) {
            dx -= MOVE_BY;
        }
        if is_key_down(KeyCode::D) {
            dx += MOVE_BY;
        }
        if is_key_down(KeyCode::W) {
            dy -= MOVE_BY;
        }
        if is_key_down(KeyCode::S) {
            dy += MOVE_BY;
        }
        let v = Vec2::new(dx, dy);
        if let Some(n) = v.try_normalize() {
            dx = n.x;
            dy = n.y;
        }
        let ox = state.x;
        let oy = state.y;
        state.x += dx;
        state.y += dy;
        fix_position_within_bounds(&mut state.x, &mut state.y);
        let true_diff_x = state.x - ox;
        let true_diff_y = state.y - oy;
        if true_diff_x != 0.0 || true_diff_y != 0.0 {
            send_message(GameInputMessage::Move { mx: true_diff_x, my: true_diff_y });
        }

        let mut remove = false;
        for (id, (color, pos)) in state.other_players.iter() {
            if *id == my_id {
                remove = true;
                continue;
            }
            draw_circle(pos.0, pos.1, PLAYER_SIZE, *color);
        }
        if remove {
            state.other_players.remove(&my_id);
        }

        draw_circle(state.x, state.y, PLAYER_SIZE, state.color);
        draw_circle_lines(state.x, state.y, PLAYER_SIZE, 3.0, WHITE);
        next_frame().await
    }
}
