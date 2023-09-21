use macroquad::prelude::*;
use sapp_jsutils::JsObject;

extern "C" {
    fn hi_from_wasm(jsobj: JsObject);
}

#[no_mangle]
pub extern "C" fn hi_rust(js_object: JsObject) {
    let mut message = String::new();

    js_object.to_string(&mut message);
    miniquad::warn!("printing from rust: {}", message);
}

#[macroquad::main("BasicShapes")]
async fn main() {
    unsafe {
        let obj = JsObject::string("eeee123");
        hi_from_wasm(obj);
    }
    loop {
        clear_background(RED);

        draw_line(40.0, 40.0, 100.0, 200.0, 15.0, BLUE);
        draw_rectangle(screen_width() / 2.0 - 60.0, 100.0, 120.0, 60.0, GREEN);
        draw_circle(screen_width() - 30.0, screen_height() - 30.0, 15.0, YELLOW);

        draw_text("IT WORKS!", 20.0, 20.0, 30.0, DARKGRAY);

        next_frame().await
    }
}
