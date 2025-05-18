//use wasm_bindgen::JsValue;
use web_sys::CanvasRenderingContext2d;
use std::sync::atomic::{AtomicU32, Ordering};
use std::collections::HashMap;

use crate::math::Vector2;

static NEXT_SQUARE_INDEX: AtomicU32 = AtomicU32::new(0);

struct KeyframeChunk {
    start_time: f64,
    end_time: f64,
    keyframes: Vec<(f64, f64, f64)>, // (time, x, y)
}

impl KeyframeChunk {
    fn new(start_time: f64, end_time: f64) -> KeyframeChunk {
        KeyframeChunk {
            start_time,
            end_time,
            keyframes: Vec::new(),
        }
    }

    fn add_keyframe(&mut self, time: f64, x: f64, y: f64) {
        // 시간이 청크 범위 내에 있는지 확인
        if time >= self.start_time && time <= self.end_time {
            self.keyframes.push((time, x, y));
        }
    }

    fn sort_keyframes(&mut self) {
        self.keyframes.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    }
}

pub struct KeyframeStore {
    chunk_size: f64,
    loaded_chunks: HashMap<u32, KeyframeChunk>,
    total_duration: f64,
    pattern_fn: Box<dyn Fn(f64, f64) -> Vec<(f64, f64, f64)>>,  // Rust 함수
}

impl KeyframeStore {
    pub fn new<F>(chunk_size: f64, total_duration: f64, pattern_fn: F) -> Self
    where
        F: 'static + Fn(f64, f64) -> Vec<(f64, f64, f64)>,
    {
        Self {
            chunk_size,
            loaded_chunks: HashMap::new(),
            total_duration,
            pattern_fn: Box::new(pattern_fn),
        }
    }

    // 특정 시간대의 청크 ID 계산
    fn get_chunk_id(&self, time: f64) -> u32 {
        (time / self.chunk_size).floor() as u32
    }

    fn ensure_chunk_loaded(&mut self, time: f64) {
        let chunk_id = self.get_chunk_id(time);
        if !self.loaded_chunks.contains_key(&chunk_id) {
            let start_time = chunk_id as f64 * self.chunk_size;
            let end_time = start_time + self.chunk_size;

            let mut chunk = KeyframeChunk::new(start_time, end_time);
            let frames = (self.pattern_fn)(start_time, end_time);

            for (t, x, y) in frames {
                chunk.add_keyframe(t, x, y);
            }

            chunk.sort_keyframes();

            if self.loaded_chunks.len() >= 5 {
                let oldest_chunk_id = *self.loaded_chunks.keys()
                    .min_by(|a, b| {
                        let a_dist = (chunk_id as i32 - **a as i32).abs();
                        let b_dist = (chunk_id as i32 - **b as i32).abs();
                        b_dist.cmp(&a_dist)
                    })
                    .unwrap();
                self.loaded_chunks.remove(&oldest_chunk_id);
            }

            self.loaded_chunks.insert(chunk_id, chunk);
        }
    }


    // 특정 시간의 키프레임 보간 위치 계산
    fn get_interpolated_position(&mut self, time: f64) -> Vector2 {
        // 시간이 총 길이를 초과하면 루프
        let t = time % self.total_duration;
        
        // 해당 시간이 포함된 청크 로드
        self.ensure_chunk_loaded(t);
        
        // 현재 청크 ID
        let chunk_id = self.get_chunk_id(t);
        
        // 청크 내에서 시간에 맞는 키프레임 찾기
        let frames;
        let chunk_end_time;
        
        {
            let chunk = self.loaded_chunks.get(&chunk_id).unwrap();
            frames = chunk.keyframes.clone(); // 키프레임 데이터 복사
            chunk_end_time = chunk.end_time; // 끝 시간 복사
        }
        
        // 청크에 키프레임이 없는 경우 인접 청크 확인
        if frames.is_empty() {
            // 이전 청크 로드 시도
            if chunk_id > 0 {
                self.ensure_chunk_loaded((chunk_id - 1) as f64 * self.chunk_size);
            }
            
            // 다음 청크 로드 시도
            self.ensure_chunk_loaded((chunk_id + 1) as f64 * self.chunk_size);
            
            // 여전히 키프레임이 없으면 기본값 반환
            return Vector2::new(0.0, 0.0);
        }
        
        // 이전/다음 키프레임 찾기
        let mut prev_frame = frames[0];
        let mut next_frame = frames[0];
        
        for i in 0..frames.len() {
            if frames[i].0 <= t {
                prev_frame = frames[i];
            }
            if frames[i].0 >= t {
                next_frame = frames[i];
                break;
            }
        }
        
        // 같은 프레임이면 보간 없이 바로 반환
        if prev_frame.0 == next_frame.0 {
            return Vector2::new(prev_frame.1, prev_frame.2);
        }
        
        // 시간이 청크 끝에 있으면 다음 청크 로드
        if next_frame.0 == chunk_end_time {
            self.ensure_chunk_loaded(chunk_end_time);
            let next_chunk_id = self.get_chunk_id(chunk_end_time);
            
            if let Some(next_chunk) = self.loaded_chunks.get(&next_chunk_id) {
                if !next_chunk.keyframes.is_empty() {
                    next_frame = next_chunk.keyframes[0];
                }
            }
        }
        
        // 위치 보간
        let factor = if next_frame.0 != prev_frame.0 {
            (t - prev_frame.0) / (next_frame.0 - prev_frame.0)
        } else {
            0.0
        };
        
        let x = prev_frame.1 + (next_frame.1 - prev_frame.1) * factor;
        let y = prev_frame.2 + (next_frame.2 - prev_frame.2) * factor;
        
        Vector2::new(x, y)
    }
}

// 사각형 객체 - 이전과 유사하지만 키프레임 저장소 사용
pub struct LazySquareObject {
    index: u32,
    size: f64,
    color: String,
    keyframe_store: KeyframeStore,
    current_time: f64,
    cached_x: f64,
    cached_y: f64,
}

impl LazySquareObject {
    pub fn new(keyframe_store: KeyframeStore, size: f64, color: &str) -> LazySquareObject {
        let index = NEXT_SQUARE_INDEX.fetch_add(1, Ordering::SeqCst);
        LazySquareObject {
            index,
            size,
            color: color.to_string(),
            keyframe_store,
            current_time: 0.0,
            cached_x: 0.0,
            cached_y: 0.0,
        }
    }

    /// 고유 인덱스 반환
    pub fn index(&self) -> u32 {
        self.index
    }

    /// 애니메이션 시간 진행
    pub fn update(&mut self, delta_time: f64) {
        self.current_time += delta_time;
        let position = self.keyframe_store.get_interpolated_position(self.current_time);
        self.cached_x = position.x;
        self.cached_y = position.y;
    }

    /// 현재 시간의 보간된 위치에 사각형 렌더링
    pub fn render(&mut self, context: &CanvasRenderingContext2d) {
        use wasm_bindgen::JsValue;
        context.set_fill_style(&JsValue::from_str(&self.color));
        context.fill_rect(self.cached_x, self.cached_y, self.size, self.size);
    }

    /// 현재 X 좌표 가져오기
    pub fn current_x(&self) -> f64 {
        self.cached_x
    }

    /// 현재 Y 좌표 가져오기
    pub fn current_y(&self) -> f64 {
        self.cached_y
    }

    pub fn get_size(&self) -> f64 {
        self.size
    }
    
    pub fn reset(&mut self) {
        self.current_time = 0.0;
    }
}