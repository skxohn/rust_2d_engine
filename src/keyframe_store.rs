use std::{num::NonZero, sync::{Arc, RwLock}};
use lru::LruCache;

use crate::{keyframe::KeyframeChunk, keyframe_database::KeyframeDatabase, math::Vector2};

const MAX_CHUNKS: usize = 2;
pub struct KeyframeStore {
    object_id: String,
    chunk_size: f64,
    total_duration: f64,
    loaded_chunks: Arc<RwLock<LruCache<u32, KeyframeChunk>>>,
    keyframe_db: Arc<KeyframeDatabase>,
}

impl KeyframeStore {
    /// Create a new KeyframeStore
    pub fn new(
        object_id: String,
        chunk_size: f64,
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

    /// Fetch a chunk from the database and cache it
    pub async fn fetch_data(&self, time: f64) -> Result<(), idb::Error> {
        let t = time % self.total_duration;
        let chunk_idx = (t / self.chunk_size).floor() as u32;

        // let msg = format!("KeyframeStore::fetch_data - time: {}, chunk_idx: {}", time, chunk_idx);
        // web_sys::console::log_1(&msg.into());

        // 읽기 잠금을 통해 캐시에 이미 있는지 확인
        {
            let cache = self.loaded_chunks.read().unwrap();
            if cache.contains(&chunk_idx) {
                return Ok(());
            }
        } // 읽기 잠금 해제

        // IndexedDB에서 로딩
        let chunk = self
            .keyframe_db
            .load_chunk(&self.object_id, chunk_idx)
            .await?;

        // 쓰기 잠금을 통해 캐시에 삽입 (자동으로 LRU 제거)
        {
            let mut cache = self.loaded_chunks.write().unwrap();
            cache.put(chunk_idx, chunk);
        } // 쓰기 잠금 해제

        Ok(())
    }

    /// Get interpolated position at a given time (synchronous)
    /// Assumes the relevant chunk has already been loaded via fetch_data
    pub fn get_interpolated_position(&self, time: f64) -> Option<Vector2> {
        let t = time % self.total_duration;
        let chunk_idx = (t / self.chunk_size).floor() as u32;

        // 읽기 잠금으로 데이터 접근
        let mut cache = self.loaded_chunks.write().unwrap(); // write() 대신 read()를 사용해도 되지만 get_mut를 위해 write 사용
        cache.get_mut(&chunk_idx).map(|chunk| chunk.interpolate(t))
    }
}