mod animation_frame;
mod squre_object;
mod math;
mod input;
mod engine;
mod lazy_squre_object;
mod keyframe;
mod keyframe_database;
mod keyframe_store;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();
    Ok(())
}
