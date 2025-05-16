use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use web_sys::CanvasRenderingContext2d;
use std::sync::atomic::{AtomicU32, Ordering};

static NEXT_SQUARE_INDEX: AtomicU32 = AtomicU32::new(0);

#[wasm_bindgen]
pub struct Keyframe {
    time: f64,
    x: f64,
    y: f64,
}

#[wasm_bindgen]
impl Keyframe {
    #[wasm_bindgen(constructor)]
    pub fn new(time: f64, x: f64, y: f64) -> Keyframe {
        Keyframe { time, x, y }
    }

    #[wasm_bindgen(getter)]
    pub fn time(&self) -> f64 { self.time }
    #[wasm_bindgen(getter)]
    pub fn x(&self) -> f64 { self.x }
    #[wasm_bindgen(getter)]
    pub fn y(&self) -> f64 { self.y }
}

#[wasm_bindgen]
pub struct SquareObject {
    index: u32,
    size: f64,
    color: String,
    keyframes: Vec<Keyframe>,
    current_time: f64,
}

#[wasm_bindgen]
impl SquareObject {
    #[wasm_bindgen(constructor)]
    pub fn new(keyframes: Vec<Keyframe>, size: f64, color: &str) -> SquareObject {
        assert!(keyframes.len() >= 2, "At least two keyframes required");
        let index = NEXT_SQUARE_INDEX.fetch_add(1, Ordering::SeqCst);
        SquareObject {
            index,
            size,
            color: color.to_string(),
            keyframes,
            current_time: 0.0,
        }
    }

    /// Unique index for this square
    #[wasm_bindgen(getter)]
    pub fn index(&self) -> u32 {
        self.index
    }

    /// Advance animation by delta_time seconds
    pub fn update(&mut self, delta_time: f64) {
        self.current_time += delta_time;
        let end_time = self.keyframes.last().unwrap().time();
        // Always loop
        self.current_time %= end_time;
    }

    /// Render the square at interpolated position, with fixed size and color
    pub fn render(&self, context: &CanvasRenderingContext2d) {
        let t = self.current_time;
        let frames = &self.keyframes;
        let len = frames.len();
        let end_time = frames[len - 1].time();

        // Determine prev and next keyframes, including wrap-around
        let (prev, next, span, elapsed) = if t < frames[0].time() {
            // Before first keyframe: wrap from last to first
            let prev = &frames[len - 1];
            let next = &frames[0];
            // Total span across loop
            let span = (end_time - prev.time()) + next.time();
            // Elapsed since prev
            let elapsed = (end_time - prev.time()) + t;
            (prev, next, span, elapsed)
        } else {
            // Between keyframes
            let mut i = 0;
            while i + 1 < len && frames[i + 1].time() < t {
                i += 1;
            }
            let prev = &frames[i];
            let next = &frames[(i + 1) % len];
            let span = next.time() - prev.time();
            let elapsed = t - prev.time();
            (prev, next, span, elapsed)
        };

        let factor = if span > 0.0 { elapsed / span } else { 0.0 };

        // Interpolate position only
        let x = prev.x() + (next.x() - prev.x()) * factor;
        let y = prev.y() + (next.y() - prev.y()) * factor;

        context.set_fill_style(&JsValue::from_str(&self.color));
        context.fill_rect(x, y, self.size, self.size);
    }

    pub fn reset(&mut self) {
        self.current_time = 0.0;
    }

    pub fn current_x(&self) -> f64 {
        let t = self.current_time;
        let frames = &self.keyframes;
        let len = frames.len();
        let end_t = frames[len-1].time();
        let t = t % end_t;

        let mut i = 0;
        while i+1 < len && frames[i+1].time() < t { i += 1; }
        let prev = &frames[i];
        let next = &frames[(i+1) % len];
        let span = (next.time() - prev.time()).rem_euclid(end_t);
        let elapsed = (t - prev.time()).rem_euclid(end_t);
        let factor = if span > 0.0 { elapsed/span } else { 0.0 };

        prev.x() + (next.x() - prev.x())*factor
    }

    pub fn current_y(&self) -> f64 {
        let t = self.current_time;
        let frames = &self.keyframes;
        let len = frames.len();
        let end_t = frames[len-1].time();
        let t = t % end_t;

        let mut i = 0;
        while i+1 < len && frames[i+1].time() < t { i += 1; }
        let prev = &frames[i];
        let next = &frames[(i+1) % len];
        let span = (next.time() - prev.time()).rem_euclid(end_t);
        let elapsed = (t - prev.time()).rem_euclid(end_t);
        let factor = if span > 0.0 { elapsed/span } else { 0.0 };

        prev.y() + (next.y() - prev.y())*factor
    }

    pub fn get_size(&self) -> f64 {
        self.size
    }
}
