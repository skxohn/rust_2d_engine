pub struct AABB {
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
}

impl AABB {
    pub fn new(x: f64, y: f64, size: f64) -> Self {
        Self {
            min_x: x,
            min_y: y,
            max_x: x + size,
            max_y: y + size,
        }
    }
    
    pub fn contains_point(&self, x: f64, y: f64) -> bool {
        x >= self.min_x && x <= self.max_x && y >= self.min_y && y <= self.max_y
    }
}