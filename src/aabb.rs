pub struct AABB {
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
}

impl AABB {
    pub fn new(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Self {
        Self {
            min_x: min_x,
            min_y: min_y,
            max_x: max_x,
            max_y: max_y,
        }
    }
    
    pub fn contains_point(&self, x: f64, y: f64) -> bool {
        x >= self.min_x && x <= self.max_x && y >= self.min_y && y <= self.max_y
    }

    pub fn intersects(&self, other: &AABB) -> bool {
        !(self.max_x < other.min_x
          || self.min_x > other.max_x
          || self.max_y < other.min_y
          || self.min_y > other.max_y)
    }
}