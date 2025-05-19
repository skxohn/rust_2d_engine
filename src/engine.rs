use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use wasm_bindgen_futures::JsFuture;
use web_sys::window;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, Window};
use std::{cell::RefCell, rc::Rc};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::aabb::AABB;
use crate::animation_frame;
use crate::keyframe::Keyframe;
use crate::keyframe::KeyframeChunk;
use crate::keyframe_database::KeyframeDatabase;
use crate::squre_object;
use crate::input;
use crate::squre_object::SquareObject;

use std::collections::VecDeque;

static NEXT_SQUARE_INDEX: AtomicU32 = AtomicU32::new(0);

enum EngineTask {
    FetchData,
    UpdateAndRender(f64),
}

#[wasm_bindgen]
pub struct Rust2DEngine {
    window: Rc<Window>,
    window_width: f64,
    window_height: f64,
    viewport: AABB,
    context: CanvasRenderingContext2d,
    last_frame_time: f64,
    objects: RefCell<Vec<squre_object::SquareObject>>,
    input_handler: input::InputHandler,
    keyframe_db: Arc<KeyframeDatabase>,
    task_queue: Rc<RefCell<VecDeque<EngineTask>>>,
}

#[wasm_bindgen]
impl Rust2DEngine {
    #[wasm_bindgen(constructor)]
    pub async fn new(canvas_id: &str) -> Result<Rust2DEngine, JsValue> {
        let window = web_sys::window().ok_or_else(|| JsValue::from_str("no global `window`"))?;
        let document = window.document().ok_or_else(|| JsValue::from_str("no `document`"))?;
        let canvas_el = document
            .get_element_by_id(canvas_id)
            .ok_or_else(|| JsValue::from_str("canvas not found"))?
            .dyn_into::<HtmlCanvasElement>()?;

        let context = canvas_el
            .get_context("2d")?
            .ok_or_else(|| JsValue::from_str("failed to get 2d context"))?
            .dyn_into::<CanvasRenderingContext2d>()?;
        let last_frame_time = window.performance().unwrap().now();
        let input_handler = input::InputHandler::new(&canvas_el)?;
        let keyframe_db = KeyframeDatabase::new()
            .await
            .map_err(|e| {
                JsValue::from_str(&format!("KeyframeDatabase init failed: {}", e))
            })?;
        let task_queue = Rc::new(RefCell::new(VecDeque::new()));
        let (width, height) = Rust2DEngine::get_window_inner_size(&window.clone());
        let viewport = AABB::new (0.0, 0.0, width as f64, height as f64);
        Ok(Rust2DEngine {
            window: Rc::new(window),
            window_width: width.into(),
            window_height: height.into(),
            viewport: viewport,
            context,
            last_frame_time,
            objects: RefCell::new(Vec::new()),
            input_handler,
            keyframe_db: keyframe_db,
            task_queue: task_queue,
        })
    }

    #[wasm_bindgen]
    pub async fn run(self) -> Result<(), JsValue> {
        let engine = Rc::new(RefCell::new(self));
        let task_queue = engine.borrow().task_queue.clone();

        // Initial data fetch
        {
            let engine_clone = engine.clone();
            engine_clone.borrow_mut().fetch_data().await?;
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

        // Start the task processing loop
        Self::start_task_loop(engine);

        Ok(())
    }


    fn start_task_loop(engine: Rc<RefCell<Self>>) {
        spawn_local(async move {
            loop {
                let task_opt = {
                    let eng_ref = engine.borrow();
                    let mut queue_ref = eng_ref.task_queue.borrow_mut();
                    queue_ref.pop_front()
                };

                if let Some(task) = task_opt {
                    let mut eng = engine.borrow_mut();
                    match task {
                        EngineTask::FetchData => {
                            if let Err(e) = eng.fetch_data().await {
                                web_sys::console::error_1(&e);
                            }
                        }
                        EngineTask::UpdateAndRender(delta) => {
                            let mouse_pressed = eng.input_handler.is_mouse_button_pressed(0)
                                || eng.input_handler.is_mouse_button_pressed(1)
                                || eng.input_handler.is_mouse_button_pressed(2);
                            if !mouse_pressed {
                                if let Err(e) = eng.update(delta) {
                                    web_sys::console::error_1(&e);
                                }
                                if let Err(e) = eng.render() {
                                    web_sys::console::error_1(&e);
                                }
                                Rust2DEngine::update_hit_indices_display("None");
                            } else {
                                let pos = eng.input_handler.get_mouse_position();
                                let hits = eng.hit_indices(pos.x, pos.y);
                                let hits_str = if hits.is_empty() {
                                    "None".to_string()
                                } else {
                                    hits.iter()
                                        .map(|i| i.to_string())
                                        .collect::<Vec<_>>()
                                        .join(", ")
                                };
                                Rust2DEngine::update_hit_indices_display(&hits_str);
                            }
                            let fps = if delta > 0.0 { 1000.0 / delta } else { 0.0 };
                            Rust2DEngine::update_fps_display(fps);
                        }
                    }
                }

                gloo_timers::future::TimeoutFuture::new(1).await;
            }
        });
    }

    pub fn update_hit_indices_display(text: &str) {
        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
            if let Some(el) = doc.get_element_by_id("hit-indices") {
                el.set_inner_html(text);
            }
        }
    }

    pub fn update_fps_display(fps: f64) {
        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
            if let Some(el) = doc.get_element_by_id("fps") {
                el.set_inner_html(&format!("{:.1} FPS", fps));
            }
        }
    }

    async fn fetch_data(&mut self) -> Result<(), JsValue> {
        let mut objs = self.objects.borrow_mut();
        for obj in objs.iter_mut() {
            obj.fetch_data().await?;
        }
        Ok(())
    }

    fn update(&mut self, delta_time: f64) -> Result<(), JsValue>{
        let mut objs = self.objects.borrow_mut();
        for obj in objs.iter_mut() {
            obj.update(delta_time)?;
        }
        Ok(())
    }

    fn render(&mut self) -> Result<(), JsValue> {
        let bg_color = JsValue::from_str("#6C5B7B");
        self.context.set_fill_style(&bg_color);
        self.context
            .fill_rect(0.0, 0.0, self.window_width as f64, self.window_height as f64);
        let objs = self.objects.get_mut();
        for obj in objs.iter_mut() {
            let bbox = AABB::new(
                    obj.current_x(), 
                    obj.current_y(), 
                    obj.current_x() + obj.get_size(),
                    obj.current_y() + obj.get_size(),
                );
            if !bbox.intersects(&self.viewport) {
                continue;
            }
            obj.render(&self.context)?;
        }
        Ok(())
    }

    fn get_window_inner_size(window: &Window) -> (u32, u32) {
        let width = window
            .inner_width()
            .expect("Failed to get window's inner width")
            .as_f64()
            .expect("Failed to convert window's inner width to f64")
            as u32;

        let height = window
            .inner_height()
            .expect("Failed to get window's inner height")
            .as_f64()
            .expect("Failed to convert window's inner height to f64")
            as u32;

        (width, height)
    }

    pub fn hit_indices(&self, x: f64, y: f64) -> Vec<u32> {
        let objs = self.objects.borrow();
        
        objs.iter()
            .filter_map(|obj| {
                let bbox = AABB::new(
                    obj.current_x(), 
                    obj.current_y(), 
                    obj.current_x() + obj.get_size(),
                    obj.current_y() + obj.get_size(),
                );
                
                if bbox.contains_point(x, y) {
                    Some(obj.object_id())
                } else {
                    None
                }
            })
            .collect()
    }

    #[wasm_bindgen]
    pub async fn generate_objects(
        &mut self,
        total_objects: u32,
        frames_per_object: u32,
        size: f64,
    ) -> Result<(), JsValue> {
        let (width, height) = Rust2DEngine::get_window_inner_size(&self.window);
        let width_f32 = width as f32;
        let height_f32 = height as f32;
        let size_f32 = size as f32;

        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let loading_el = document.get_element_by_id("loading").unwrap();

        let rng = js_sys::Math::random;

        for idx in 0..total_objects {
            {
                let promise = js_sys::Promise::new(&mut |resolve, _reject| {
                    self.window
                        .request_animation_frame(&resolve)
                        .unwrap();
                });
                JsFuture::from(promise).await?;
            }

            let progress_ratio = (idx + 1) as f32 / total_objects as f32;
            let percentage = progress_ratio * 100.0;
            let progress_text = format!(
                "Creating objects: {} / {} ({:.1}%)",
                idx + 1,
                total_objects,
                percentage
            );

            loading_el.set_inner_html(&progress_text);

            let object_id = NEXT_SQUARE_INDEX.fetch_add(1, Ordering::SeqCst);
            let chunk_size = 10_000.0 + (rng() as f32 * 310.0).floor() * 100.0;

            let color = format!("#{:06x}", (rng() * 0xFFFFFF as f64).floor() as u32);
            let mut chunks: Vec<KeyframeChunk> = Vec::new();

            let mut current_chunk: Vec<Keyframe> = Vec::new();
            let mut current_start_time = 0.0f32;

            let mut t = 0.0f32;
            let x0 = rng() as f32 * (width_f32 - size_f32);
            let y0 = rng() as f32 * (height_f32 - size_f32);
            current_chunk.push(Keyframe::new(t, x0, y0));

            for _ in 0..frames_per_object {
                t += rng() as f32 * 1000.0;
                let x = rng() as f32 * (width_f32 - size_f32);
                let y = rng() as f32 * (height_f32 - size_f32);
                let keyframe = Keyframe::new(t, x, y);

                if t >= current_start_time + chunk_size {
                    let chunk = KeyframeChunk::new(
                        &format!("{}_{}", object_id, (current_start_time / chunk_size).floor() as u32),
                        current_chunk.first().unwrap().time(),
                        current_chunk.last().unwrap().time(),
                        current_chunk,
                    );
                    chunks.push(chunk);

                    current_chunk = Vec::new();
                    current_start_time += chunk_size;
                }

                current_chunk.push(keyframe);
            }

            if !current_chunk.is_empty() {
                let chunk = KeyframeChunk::new(
                    &format!("{}_{}", object_id, (current_start_time / chunk_size).floor() as u32),
                    current_chunk.first().unwrap().time(),
                    current_chunk.last().unwrap().time(),
                    current_chunk,
                );
                chunks.push(chunk);
            }

            let square = SquareObject::new(
                object_id,
                size,
                &color,
                chunks,
                chunk_size,
                Arc::clone(&self.keyframe_db)
            ).await;

            self.objects.borrow_mut().push(square);
        }

        loading_el.set_inner_html("Preprocessing...");

        let engine = Rc::new(RefCell::new(self));
        {
            let engine_clone = engine.clone();
            engine_clone.borrow_mut().fetch_data().await?;
        }

        Ok(())
    }

}
