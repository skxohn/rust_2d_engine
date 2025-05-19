use std::{num::NonZero, sync::{Arc, RwLock}};
use lru::LruCache;

use crate::{keyframe::KeyframeChunk, keyframe_database::KeyframeDatabase, math::Vector2};

const MAX_CHUNKS: usize = 2;
pub struct KeyframeStore {
    object_id: String,
    chunk_size: f32,
    total_duration: f64,
    loaded_chunks: Arc<RwLock<LruCache<u32, KeyframeChunk>>>,
    keyframe_db: Arc<KeyframeDatabase>,
}

impl KeyframeStore {
    pub fn new(
        object_id: String,
        chunk_size: f32,
        total_duration: f64,
        keyframe_db: Arc<KeyframeDatabase>,
    ) -> Self {
        KeyframeStore {
            object_id,
            chunk_size,
            loaded_chunks: Arc::new(RwLock::new(LruCache::new(NonZero::new(MAX_CHUNKS).unwrap()))),
            total_duration,
            keyframe_db,
        }
    }

    pub async fn fetch_data(&self, time: f64) -> Result<(), idb::Error> {
        let t = time % self.total_duration;
        let chunk_idx = (t / self.chunk_size as f64).floor() as u32;

        {
            let cache = self.loaded_chunks.read().unwrap();
            if cache.contains(&chunk_idx) {
                return Ok(());
            }
        }

        let chunk = self
            .keyframe_db
            .load_chunk(&self.object_id, chunk_idx)
            .await?;

        {
            let mut cache = self.loaded_chunks.write().unwrap();
            cache.put(chunk_idx, chunk);
        }

        Ok(())
    }

    pub fn get_interpolated_position(&self, time: f64) -> Option<Vector2> {
        let t = time % self.total_duration;
        let chunk_idx = (t / self.chunk_size as f64).floor() as u32;

        let mut cache = self.loaded_chunks.write().unwrap();
        cache.get_mut(&chunk_idx).map(|chunk| chunk.interpolate(t as f32))
    }
}