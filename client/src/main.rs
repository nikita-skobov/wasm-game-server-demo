use macroquad::prelude::*;
use sapp_jsutils::JsObject;
use std::sync::{OnceLock, Mutex};

fn ws_messages() -> &'static mut Mutex<Vec<String>> {
    static mut ARRAY: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
    unsafe {
        ARRAY.get_or_init(|| Mutex::new(vec![]));
        ARRAY.get_mut().unwrap()
    }
}

extern "C" {
    fn hi_from_wasm(jsobj: JsObject);
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

pub fn handle_server_events() {
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
        miniquad::warn!("WS: {}", ev);
    }
}

#[macroquad::main("BasicShapes")]
async fn main() {
    unsafe {
        let obj = JsObject::string("eeee123");
        hi_from_wasm(obj);
    }
    loop {
        clear_background(RED);
        handle_server_events();

        draw_line(40.0, 40.0, 100.0, 200.0, 15.0, BLUE);
        draw_rectangle(screen_width() / 2.0 - 60.0, 100.0, 120.0, 60.0, GREEN);
        draw_circle(screen_width() - 30.0, screen_height() - 30.0, 15.0, YELLOW);

        draw_text("IT WORKS!", 20.0, 20.0, 30.0, DARKGRAY);

        next_frame().await
    }
}
