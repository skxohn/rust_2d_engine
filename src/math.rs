use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Vector2 {
    pub x: f64,
    pub y: f64,
}

#[wasm_bindgen]
impl Vector2 {
     #[wasm_bindgen(constructor)]
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}