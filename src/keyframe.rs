use serde::{Deserialize, Serialize};

use crate::math::Vector2;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Keyframe {
    time: f32,
    x: f32,
    y: f32,
}

impl Keyframe {
    pub fn new(time: f32, x: f32, y: f32) -> Keyframe {
        Keyframe { time, x, y }
    }

    pub fn time(&self) -> f32 { self.time }
    pub fn x(&self) -> f32 { self.x }
    pub fn y(&self) -> f32 { self.y }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KeyframeChunk {
    object_chunk_id: String,
    start_time: f32,
    end_time: f32,
    keyframes: Vec<Keyframe>,
}

impl KeyframeChunk {
    pub fn new(object_chunk_id: &str, start_time: f32, end_time: f32, keyframes: Vec<Keyframe>) -> Self {
        Self {
            object_chunk_id: object_chunk_id.to_string(),
            start_time,
            end_time,
            keyframes: keyframes,
        }
    }

    // pub fn add_keyframe(&mut self, time: f32, x: f32, y: f32) {
    //     if time >= self.start_time && time <= self.end_time {
    //         self.keyframes.push(Keyframe { time, x, y });
    //     }
    // }

    pub fn interpolate(&self, time: f32) -> Vector2 {
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
            return Vector2::new(k.x().into(), k.y().into());
        }

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
                return Vector2::new(x.into(), y.into());
            }
            prev = next;
        }

        // If time is after the last keyframe, return last position
        let last = self.keyframes.last().unwrap();
        Vector2::new(last.x().into(), last.y().into())
    }

    // pub fn log_contents(&self) {
    //     let header = format!(
    //         "KeyframeChunk [{}] (start: {:.2}, end: {:.2}, total: {})",
    //         self.object_chunk_id,
    //         self.start_time,
    //         self.end_time,
    //         self.keyframes.len()
    //     );
    //     web_sys::console::log_1(&header.into());

    //     for (i, kf) in self.keyframes.iter().enumerate() {
    //         let line = format!(
    //             "  [{}] time: {:.2}, x: {:.2}, y: {:.2}",
    //             i,
    //             kf.time(),
    //             kf.x(),
    //             kf.y()
    //         );
    //         web_sys::console::log_1(&line.into());
    //     }
    // }

    pub fn end_time(&self) -> f32 {
        self.end_time
    }
}
