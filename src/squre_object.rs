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
            keyframe_store: keyframe_store,
        }
    }

    /// Unique index for this square
    pub fn index(&self) -> u32 {
        self.index
    }

    pub async fn fetch_data(&mut self) -> Result<(), JsValue> {
        let _ = self.keyframe_store.fetch_data(self.current_time).await;
        Ok(())
    }

    /// Advance animation by delta_time seconds
    pub fn update(&mut self, delta_time: f64) -> Result<(), JsValue> {
        self.current_time = (self.current_time + delta_time) % self.total_duration;
        if let Some(pos) = self.keyframe_store.get_interpolated_position(self.current_time) {
            self.cached_x = pos.x;
            self.cached_y = pos.y;
        }
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
