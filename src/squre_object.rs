use wasm_bindgen::JsValue;
use web_sys::CanvasRenderingContext2d;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
//use std::rc::Rc;

use crate::keyframe::Keyframe;
use crate::keyframe_store::KeyframeStore;
use crate::keyframe_database::KeyframeDatabase;
use crate::math::Vector2;

static NEXT_SQUARE_INDEX: AtomicU32 = AtomicU32::new(0);

pub struct SquareObject {
    index: u32,
    size: f64,
    color: String,
    current_time: f64,
    total_duration: f64,
    cached_x: f64,
    cached_y: f64,
    keyframes: Vec<Keyframe>,
    keyframe_store: KeyframeStore,
}

impl SquareObject {
    pub async fn new(
        keyframe_db: Arc<KeyframeDatabase>,
        keyframes: Vec<Keyframe>, 
        size: f64, 
        color: &str
    ) -> SquareObject {

        let index = NEXT_SQUARE_INDEX.fetch_add(1, Ordering::SeqCst);
        //let chunk_size = 5000.0;
        let chunk_size = 10000.0 + (js_sys::Math::floor(js_sys::Math::random() * 310.0) * 100.0);

        let total_duration = keyframes
            .last()
            .expect("keyframes non-empty")
            .time();

        let _ = keyframe_db
            .save_keyframes_sequentially(&index.to_string(), keyframes.clone(), chunk_size)
            .await;

        let keyframe_store = KeyframeStore::new(
            index.to_string(), 
            chunk_size,
            total_duration,
            keyframe_db.into(),
        );
        SquareObject {
            index,
            size,
            color: color.to_string(),
            current_time: 0.0,
            total_duration: total_duration,
            cached_x: 0.0,
            cached_y: 0.0,
            keyframes: Vec::new(),
            keyframe_store: keyframe_store,
        }
    }

    /// Unique index for this square
    pub fn index(&self) -> u32 {
        self.index
    }

    pub fn get_interpolated_position(&self, time: f64) -> Vector2 {
        let frames = &self.keyframes;
        let len = frames.len();

        if len < 2 {
            return Vector2::new(0.0, 0.0);
        }

        let end_time = frames[len - 1].time();

        // Determine prev and next keyframes, including wrap-around
        let (prev, next, span, elapsed) = if time < frames[0].time() {
            let prev = &frames[len - 1];
            let next = &frames[0];
            let span = (end_time - prev.time()) + next.time();
            let elapsed = (end_time - prev.time()) + time;
            (prev, next, span, elapsed)
        } else {
            let mut i = 0;
            while i + 1 < len && frames[i + 1].time() < time {
                i += 1;
            }
            let prev = &frames[i];
            let next = &frames[(i + 1) % len];
            let span = next.time() - prev.time();
            let elapsed = time - prev.time();
            (prev, next, span, elapsed)
        };

        let factor = if span > 0.0 { elapsed / span } else { 0.0 };

        let x = prev.x() + (next.x() - prev.x()) * factor;
        let y = prev.y() + (next.y() - prev.y()) * factor;

        Vector2::new(x, y)
    }

    pub async fn fetch_data(&mut self) -> Result<(), JsValue> {
        // let msg = "SquareObject::fetch_data";
        // web_sys::console::log_1(&msg.into());
        let _ = self.keyframe_store.fetch_data(self.current_time).await;
        Ok(())
    }

    /// Advance animation by delta_time seconds
    pub fn update(&mut self, delta_time: f64) -> Result<(), JsValue> {
        // let msg1 = format!("SquareObject::update pre - current_time: {}", self.current_time);
        // web_sys::console::log_1(&msg1.into());

        self.current_time = (self.current_time + delta_time) % self.total_duration;
        if let Some(pos) = self.keyframe_store.get_interpolated_position(self.current_time) {
            self.cached_x = pos.x;
            self.cached_y = pos.y;
        }

        // let msg = format!("SquareObject::update - current_time: {}", self.current_time);
        // web_sys::console::log_1(&msg.into());

        // let frames = &self.keyframes;
        // let len = frames.len();
        // if len > 0 {
        //     let pos = self.get_interpolated_position(self.current_time);
        //     self.cached_x = pos.x;
        //     self.cached_y = pos.y;
        // }
        Ok(())
    }

    /// Render the square at interpolated position, with fixed size and color
    pub fn render(&self, context: &CanvasRenderingContext2d) -> Result<(), JsValue>{
        context.set_fill_style(&JsValue::from_str(&self.color));
        context.fill_rect(self.cached_x, self.cached_y, self.size, self.size);
        Ok(())
    }

    pub fn reset(&mut self) {
        self.current_time = 0.0;
    }

    pub fn current_x(&self) -> f64 {
        self.cached_x
    }

    pub fn current_y(&self) -> f64 {
        self.cached_y
    }

    pub fn get_size(&self) -> f64 {
        self.size
    }
}
