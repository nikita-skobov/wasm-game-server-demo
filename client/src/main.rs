use macroquad::prelude::*;
use sapp_jsutils::JsObject;
use shared::*;
use std::sync::{OnceLock, Mutex};

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
                }
            }
            GameOutputMessage::YouAre { id } => {
                state.connected_id = Some(id);
            }
        }
    }
}

#[derive(Default)]
pub struct ClientState {
    pub connected_id: Option<u64>,
}

#[macroquad::main("BasicShapes")]
async fn main() {
    let mut state = ClientState::default();
    loop {
        let b_color = if state.connected_id.is_some() {
            BLACK
        } else {
            GRAY
        };
        clear_background(b_color);
        handle_server_events(&mut state);
        if state.connected_id.is_none() {
            draw_text("CONNECTING...", 20.0, 20.0, 48.0, WHITE);
            next_frame().await;
            continue;
        }
        if is_key_pressed(KeyCode::A) {
            send_message(GameInputMessage::Move { mx: 1.0, my: 3.5 });
        }

        // draw_line(40.0, 40.0, 100.0, 200.0, 15.0, BLUE);
        // draw_rectangle(screen_width() / 2.0 - 60.0, 100.0, 120.0, 60.0, GREEN);
        // draw_circle(screen_width() - 30.0, screen_height() - 30.0, 15.0, YELLOW);

        // draw_text("IT WORKS!", 20.0, 20.0, 30.0, DARKGRAY);

        next_frame().await
    }
}
