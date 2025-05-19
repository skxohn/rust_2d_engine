use serde::{Deserialize, Serialize};

use crate::math::Vector2;

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

    pub fn object_chunk_id(&self) -> String { self.object_chunk_id.clone() }

    pub fn interpolate(&self, time: f64) -> Vector2 {

        // web_sys::console::log_1(
        //     &format!("KeyframeChunk:interpolate() time = {:.2}, keyframes.len() = {}", time, self.keyframes.len()).into(),
        // );
        // self.log_contents();
        // If no keyframes, return zero vector
        if self.keyframes.is_empty() {
            return Vector2::new(0.0, 0.0);
        }

        // Clamp time within chunk bounds
        let t = if time < self.start_time {
            self.start_time
        } else if time > self.end_time {
            self.end_time
        } else {
            time
        };

        // If only one keyframe, return its position
        if self.keyframes.len() == 1 {
            let k = &self.keyframes[0];
            return Vector2::new(k.x(), k.y());
        }

        // web_sys::console::log_1(
        //     &format!("KeyframeChunk:interpolate() ------", ).into(),
        // );

        // Find surrounding keyframes
        let mut prev = &self.keyframes[0];
        for next in &self.keyframes[1..] {
            if t <= next.time() {
                // found the interval [prev, next]
                let span = next.time() - prev.time();
                let ratio = if span > 0.0 {
                    (t - prev.time()) / span
                } else {
                    0.0
                };
                let x = prev.x() + ratio * (next.x() - prev.x());
                let y = prev.y() + ratio * (next.y() - prev.y());
                return Vector2::new(x, y);
            }
            prev = next;
        }

        // If time is after the last keyframe, return last position
        let last = self.keyframes.last().unwrap();

        // web_sys::console::log_1(
        //     &format!("KeyframeChunk:interpolate() x = {:.2}, y = {:.2}", last.x(), last.y()).into(),
        // );
        Vector2::new(last.x(), last.y())
    }

    pub fn log_contents(&self) {
        let header = format!(
            "KeyframeChunk [{}] (start: {:.2}, end: {:.2}, total: {})",
            self.object_chunk_id,
            self.start_time,
            self.end_time,
            self.keyframes.len()
        );
        web_sys::console::log_1(&header.into());

        for (i, kf) in self.keyframes.iter().enumerate() {
            let line = format!(
                "  [{}] time: {:.2}, x: {:.2}, y: {:.2}",
                i,
                kf.time(),
                kf.x(),
                kf.y()
            );
            web_sys::console::log_1(&line.into());
        }
    }
}
