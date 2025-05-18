use std::collections::HashMap;
use std::rc::Rc;
use wasm_bindgen::JsValue;

use crate::{keyframe::KeyframeChunk, keyframe_database::KeyframeDatabase, math::Vector2};

pub struct KeyframeStore {
    object_id: String,
    chunk_size: f64,
    loaded_chunks: HashMap<u32, KeyframeChunk>,
    total_duration: f64,
    keyframe_db: Rc<KeyframeDatabase>,
}

impl KeyframeStore {
    pub fn new(
        object_id: &str,
        chunk_size: f64,
        total_duration: f64,
        keyframe_db: Rc<KeyframeDatabase>,
    ) -> Self
    {
        Self {
            object_id: object_id.to_string(),
            chunk_size,
            loaded_chunks: HashMap::new(),
            total_duration,
            keyframe_db
        }
    }

    fn get_chunk_id(&self, time: f64) -> u32 {
        (time / self.chunk_size).floor() as u32
    }

    pub async fn ensure_chunk_loaded(&mut self, time: f64) -> Result<(), idb::Error> {
        let chunk_id = self.get_chunk_id(time);

        // Already loaded?
        if self.loaded_chunks.contains_key(&chunk_id) {
            return Ok(());
        }

        let start_time = chunk_id as f64 * self.chunk_size;
        let end_time = start_time + self.chunk_size;

        // 1) Try loading from DB
        if let Some(chunk) = 
            self.keyframe_db
                .load_chunk(&self.object_id, chunk_id)
                .await?
        {
            self.loaded_chunks.insert(chunk_id, chunk);
        } else {
            // 2) Not in DB → initialize empty chunk
            let object_chunk_id = format!("{}_{}", self.object_id.clone(), chunk_id);
            let chunk = KeyframeChunk::new(&object_chunk_id, start_time, end_time);
            self.loaded_chunks.insert(chunk_id, chunk);
        }

        // 3) Evict farthest chunk if we exceed capacity
        if self.loaded_chunks.len() > 5 {
            // find the loaded chunk whose index is farthest from `chunk_id`
            let farthest = *self.loaded_chunks
                .keys()
                .max_by(|&&a, &&b| {
                    let da = (chunk_id as i32 - a as i32).abs();
                    let db = (chunk_id as i32 - b as i32).abs();
                    da.cmp(&db)
                })
                .unwrap();

            self.loaded_chunks.remove(&farthest);
        }

        Ok(())
    }

    pub async fn get_interpolated_position(&mut self, time: f64) -> Result<Vector2, JsValue> {
        // 시간이 총 길이를 초과하면 루프
        let t = time % self.total_duration;
        
        // 해당 시간이 포함된 청크 로드
        self.ensure_chunk_loaded(t).await;
        
        // 현재 청크 ID
        let chunk_id = self.get_chunk_id(t);
        
        // 청크 내에서 시간에 맞는 키프레임 찾기
        let frames;
        let chunk_end_time;
        
        {
            let chunk = self.loaded_chunks.get(&chunk_id).unwrap();
            frames = chunk.keyframes(); // 키프레임 데이터 복사
            chunk_end_time = chunk.end_time(); // 끝 시간 복사
        }
        
        // 청크에 키프레임이 없는 경우 인접 청크 확인
        if frames.is_empty() {
            // 이전 청크 로드 시도
            if chunk_id > 0 {
                self.ensure_chunk_loaded((chunk_id - 1) as f64 * self.chunk_size).await;
            }
            
            // 다음 청크 로드 시도
            self.ensure_chunk_loaded((chunk_id + 1) as f64 * self.chunk_size).await;
            
            // 여전히 키프레임이 없으면 기본값 반환
            return Ok(Vector2::new(0.0, 0.0));
        }
        
        // 이전/다음 키프레임 찾기
        let mut prev_frame = frames[0].clone();
        let mut next_frame = frames[0].clone();
        
        for i in 0..frames.len() {
            if frames[i].time() <= t {
                prev_frame = frames[i].clone();
            }
            if frames[i].time() >= t {
                next_frame = frames[i].clone();
                break;
            }
        }
        
        // 같은 프레임이면 보간 없이 바로 반환
        if prev_frame.time() == next_frame.time() {
            return Ok(Vector2::new(prev_frame.x(), prev_frame.y()));
        }
        
        // 시간이 청크 끝에 있으면 다음 청크 로드
        if next_frame.time() == chunk_end_time {
            self.ensure_chunk_loaded(chunk_end_time);
            let next_chunk_id = self.get_chunk_id(chunk_end_time);
            
            if let Some(next_chunk) = self.loaded_chunks.get(&next_chunk_id) {
                if !next_chunk.keyframes().is_empty() {
                    next_frame = next_chunk.keyframes()[0].clone();
                }
            }
        }
        
        // 위치 보간
        let factor = if next_frame.time() != prev_frame.time() {
            (t - prev_frame.time()) / (next_frame.time() - prev_frame.time())
        } else {
            0.0
        };
        
        let x = prev_frame.x() + (next_frame.x() - prev_frame.x()) * factor;
        let y = prev_frame.y() + (next_frame.y() - prev_frame.y()) * factor;
        
        Ok(Vector2::new(x, y))
    }
}