use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Keyframe {
    time: f64,
    x: f64,
    y: f64,
}

impl Keyframe {
    pub fn new(time: f64, x: f64, y: f64) -> Keyframe {
        Keyframe { time, x, y }
    }

    pub fn time(&self) -> f64 { self.time }
    pub fn x(&self) -> f64 { self.x }
    pub fn y(&self) -> f64 { self.y }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KeyframeChunk {
    object_chunk_id: String,
    start_time: f64,
    end_time: f64,
    keyframes: Vec<Keyframe>,
}

impl KeyframeChunk {
    pub fn new(object_chunk_id: &str, start_time: f64, end_time: f64) -> Self {
        Self {
            object_chunk_id: object_chunk_id.to_string(),
            start_time,
            end_time,
            keyframes: Vec::new(),
        }
    }

    pub fn add_keyframe(&mut self, time: f64, x: f64, y: f64) {
        if time >= self.start_time && time <= self.end_time {
            self.keyframes.push(Keyframe { time, x, y });
        }
    }

    fn sort_keyframes(&mut self) {
        self.keyframes
            .sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
    }

    pub fn start_time(&self) -> f64 { self.start_time }

    pub fn end_time(&self) -> f64 { self.end_time }

    pub fn keyframes(&self) -> Vec<Keyframe> { self.keyframes.clone() }
}
