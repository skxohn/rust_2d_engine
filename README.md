## Creator/Engine Engineer Assessment

## Overview

This project implements a large-scale real-time animation system for squares using WebAssembly and Rust. The system renders thousands of squares on an HTML canvas, each following the same animation pattern but with random color and time offset. When a user clicks or drags anywhere on the canvas, the system reports the index of the square under the cursor while pausing the animation.

## Features

* **1,000,000 Keyframes** generated randomly and stored in chunks.
* **Dynamic Square Instances** with random colors and per-square time offsets.
* **Linear Interpolation** between keyframes for smooth motion.
* **Pause-on-Drag** interaction that reports the index of the square under the cursor in real time.
* **Chunked IndexedDB Storage** for scalable keyframe management.
* **Efficient Memory Usage** through LRU caching and chunked loading

![screenshot](images/image.png)

## Getting Started

### Prerequisites

* Rust toolchain with `wasm-pack`
* Node.js (v14+)
* A modern browser with IndexedDB support

### Installation

```bash
# Clone repository
git clone https://github.com/skxohn/rust_2d_engine
cd rust_2d_engine

# Install dependencies
npm install
```

### Running the Demo

```bash
npm run serve
```

Navigate to `http://localhost:8081`.

### Usage

```javascript
// Import the WASM module
import('./pkg')
  .then(async wasm => {
    const loadingEl = document.getElementById('loading');
    const canvasEl  = document.getElementById('canvas');
    const hitsEl    = document.getElementById('hit-indices');

    loadingEl.style.display = 'block';
    canvasEl.style.display  = 'none';
    hitsEl.style.display    = 'none';

    const engine = await new wasm.Rust2DEngine("canvas");

    const totalObjects = 1_000;       // Number of square instances to generate
    const size         = 100;         // Size of each square (pixels)
    const keyframesPer = 1_000_000;   // Number of keyframes per square
    
    const startTime = performance.now();

    try {
        await engine.generate_objects(
          totalObjects, 
          keyframesPer, 
          size
        );
        
        const endTime = performance.now();
        const timeElapsed = (endTime - startTime) / 1000;
        console.log(`Total initialization time: ${timeElapsed.toFixed(2)} seconds`);
        console.log(`Created ${totalObjects} objects with ${keyframesPer} frames each`);

        loadingEl.style.display = 'none';
        canvasEl.style.display = 'block';
        hitsEl.style.display = 'block';

        displayMemoryUsage();

        // Start the engine loop
        engine.run();
      } catch (error) {
        console.error("Error generating objects:", error);
        loadingEl.textContent = 'Failed to generate objects!';
      }

  })
  .catch(err => {
    console.error(err);
    const loadingEl = document.getElementById('loading');
    loadingEl.textContent = 'Failed to load!';
  });

function displayMemoryUsage() {
  if (window.performance && window.performance.memory) {
    const memoryInfo = window.performance.memory;
    const memoryUsed = Math.round(memoryInfo.usedJSHeapSize / (1024 * 1024));
    const memoryLimit = Math.round(memoryInfo.jsHeapSizeLimit / (1024 * 1024));

    console.log(`Memory: ${memoryUsed}MB / ${memoryLimit}MB`);
  }
}
```

## Technical Architecture

### Technology Stack

* Frontend: HTML5, CSS3, JavaScript/TypeScript
* Animation & Computation: Rust, WebAssembly
* Storage: IndexedDB
* Build Tools: wasm-pack, webpack
* Development Environment: Node.js

### Project Structure

```plaintext
rust_2d_engine
├── src/
│   ├── animation_frame.rs
│   ├── engine.rs
│   ├── input.rs
│   ├── keyframe_database.rs
│   ├── keyframe_store.rs
│   ├── keyframe.rs
│   ├── lib.rs
│   ├── math.rs
│   └── squre_object.rs
├── Cargo.toml
├── index.html
├── index.js
├── package.json
├── README.md
└── webpack.config.js
```

### System Architecture

The application follows modular architecture:

1. **Rust Wasm Module** (`Rust2DEngine`): 
   * Manages animation state and interpolation
   * Performs hit testing for mouse interactions
   * Handles chunked keyframe storage and retrieval
   * Implements core rendering logic

2. **JavaScript Frontend**: 
   * Initializes the canvas and WASM module
   * Manages user interactions
   * Handles browser-specific APIs

3. **Data Management**: 
   * IndexedDB for persistent storage of keyframe chunks
   * LRU caching for efficient memory usage

## Technical Challenges and Solutions

### Challenge 1: Handling One Million Keyframes
**Problem**: Managing memory usage with such a large dataset.

**Solution**: Implemented chunked storage with IndexedDB and LRU caching.

- Initially implemented using a monolithic `Vec<Keyframe>`, which caused memory usage to spike as the number of objects increased.
- Improved by using IndexedDB to store keyframes and dynamically loading them in chunks as needed for playback.
- Implemented LRU (Least Recently Used) caching to delete the oldest chunks and dynamically load keyframe chunks required for playback.
- Created `save_chunks()` and `load_chunk()` functions in keyframe_database for IndexedDB storage and retrieval.
- Managed KeyframeChunks with LruCache in keyframe_store to optimize memory usage.
- Successfully prevented memory overflow issues, resulting in a stable system regardless of animation length.

```rust
// Key data structures for memory-efficient keyframe management
pub struct Keyframe {
    time: f32,
    x: f32,
    y: f32,
}

pub struct KeyframeChunk {
    object_chunk_id: String,
    start_time: f32,
    end_time: f32,
    keyframes: Vec<Keyframe>,
}

pub struct KeyframeStore {
    object_id: String,
    chunk_size: f32,
    total_duration: f64,
    loaded_chunks: Arc<RwLock<LruCache<u32, KeyframeChunk>>>,
    keyframe_db: Arc<KeyframeDatabase>,
}
```

### Challenge 2: Efficient Hit Testing
**Problem**: Determining which squares are under the cursor quickly.

**Solution**: Implemented using AABB (Axis-Aligned Bounding Box) structure.

- Initially implemented using simple loop optimization approaches.
- While performance testing showed no significant issues, the code was refactored to use AABB structures for better readability and maintainability.
- This approach resulted in clearer and more efficient hit testing logic that's easier to extend.

```rust
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

pub fn hit_indices(&self, x: f64, y: f64) -> Vec<u32> {
    let objs = self.objects.borrow();
    
    objs.iter()
        .filter_map(|obj| {
            let bbox = AABB::new(obj.current_x(), obj.current_y(), obj.get_size());
            
            if bbox.contains_point(x, y) {
                Some(obj.object_id())
            } else {
                None
            }
        })
        .collect()
}
```

### Challenge 3: Cross-Thread Communication
**Problem**: Coordinating data loading and animation rendering.

**Solution**: Created a task queue system with separated concerns and message passing.

- Initial implementation handled keyframe fetching, updating, interpolation, and rendering all within the `window.request_animation_frame()` callback function.
- This caused issues including panics when asynchronously fetching keyframes.
- Improved by separating fetch() and update()+render() into distinct tasks processed through a task queue.
- The fetch() function (loading keyframes in chunks) is called every 20ms, while update() and render() run at 60fps through `window.request_animation_frame()`.
- While it would be possible to further separate update() and render(), the current implementation works well enough to maintain as is.

```rust
pub async fn run(self) -> Result<(), JsValue> {
    let engine = Rc::new(RefCell::new(self));
    let task_queue = engine.borrow().task_queue.clone();

    // Initial data fetch
    {
        let engine_clone = engine.clone();
        engine_clone.borrow_mut().fetch_data().await?;
    }

    // Set up periodic data fetching task (every 20ms)
    {
        let task_queue = task_queue.clone();
        let closure = Closure::wrap(Box::new(move || {
            task_queue.borrow_mut().push_back(EngineTask::FetchData);
        }) as Box<dyn FnMut()>);
        window().unwrap()
            .set_interval_with_callback_and_timeout_and_arguments_0(
                closure.as_ref().unchecked_ref(),
                20,
            )
            .unwrap();
        closure.forget();
    }

    // Setup animation frame loop for update and render
    {
        let engine_clone = engine.clone();
        let task_queue = task_queue.clone();
        let window = engine.borrow().window.clone();

        let f: Rc<RefCell<dyn FnMut() -> Result<(), JsValue>>> =
            Rc::new(RefCell::new(move || {
                if let Ok(mut eng) = engine_clone.try_borrow_mut() {
                    let now = eng.window.performance().unwrap().now();
                    let delta = now - eng.last_frame_time;
                    eng.last_frame_time = now;
                    task_queue.borrow_mut().push_back(EngineTask::UpdateAndRender(delta));
                }
                Ok(())
            }));

        animation_frame::request_recursive(window, f)?;
    }

    // Start the task processing loop
    Self::start_task_loop(engine);

    Ok(())
}
```

### Challenge 4: Performance Optimization
**Problem**: Duplicate calculations occurred between keyframe interpolation, rendering, and hit testing.

**Solution**: Calculation caching for rendering and hit testing

- Initially, each function independently performed interpolation calculations.
- Time-based interpolation especially required significant computational resources.
- Improved by performing coordinate calculations needed for both rendering and hit testing only once during the update() phase and caching the results.
- By caching calculated values, we prevented redundant calculations and significantly improved performance.

```rust
// Example of linear interpolation function that benefits from caching
pub fn interpolate(&self, time: f32) -> Vector2 {
    if self.keyframes.is_empty() {
        return Vector2::new(0.0, 0.0);
    }

    // Find surrounding keyframes and interpolate
    let mut prev = &self.keyframes[0];
    for next in &self.keyframes[1..] {
        if time <= next.time() {
            let span = next.time() - prev.time();
            let ratio = if span > 0.0 {
                (time - prev.time()) / span
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
```

```rust
pub fn update(&mut self, delta_time: f64) -> Result<(), JsValue> {
    self.current_time = (self.current_time + delta_time) % self.total_duration;
    if let Some(pos) = self.keyframe_store.get_interpolated_position(self.current_time) {
        self.cached_x = pos.x;
        self.cached_y = pos.y;
    }
    Ok(())
}
```