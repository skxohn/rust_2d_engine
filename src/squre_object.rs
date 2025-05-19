use wasm_bindgen::JsValue;
use web_sys::CanvasRenderingContext2d;
use std::sync::Arc;

use crate::keyframe::{KeyframeChunk};
use crate::keyframe_store::KeyframeStore;
use crate::keyframe_database::KeyframeDatabase;

pub struct SquareObject {
    object_id: u32,
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
        object_id: u32,
        size: f64, 
        color: &str,
        chunks: Vec<KeyframeChunk>,
        chunk_size: f32,
        keyframe_db: Arc<KeyframeDatabase>,
    ) -> SquareObject {

        let total_duration = chunks
            .iter()
            .map(|chunk| chunk.end_time())
            .fold(0.0, f32::max);

        let _ = keyframe_db
            .save_keyframes_sequentially(chunks)
            .await;

        let keyframe_store = KeyframeStore::new(
            object_id.to_string(), 
            chunk_size,
            total_duration.into(),
            keyframe_db.into(),
        );
        SquareObject {
            object_id,
            size,
            color: color.to_string(),
            current_time: 0.0,
            total_duration: total_duration.into(),
            cached_x: 0.0,
            cached_y: 0.0,
            keyframe_store: keyframe_store,
        }
    }

    /// Unique index for this square
    pub fn object_id(&self) -> u32 {
        self.object_id
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

    // pub fn reset(&mut self) {
    //     self.current_time = 0.0;
    // }

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
