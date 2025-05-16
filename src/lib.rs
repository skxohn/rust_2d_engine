mod animation_frame;
mod squre_object;
mod math;
mod input;

use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, Window};

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();
    Ok(())
}

#[wasm_bindgen]
pub struct Rust2DEngine {
    window: Rc<Window>,
    context: CanvasRenderingContext2d,
    last_frame_time: f64,
    objects: Vec<squre_object::SquareObject>,
    input_handler: input::InputHandler,
}

#[wasm_bindgen]
impl Rust2DEngine {
    #[wasm_bindgen(constructor)]
    pub fn new(canvas_id: &str) -> Result<Rust2DEngine, JsValue> {
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

        Ok(Rust2DEngine {
            window: Rc::new(window),
            context,
            last_frame_time,
            objects: Vec::new(),
            input_handler,
        })
    }

    #[wasm_bindgen]
    pub fn run(self) -> Result<(), JsValue> {
        let engine = Rc::new(RefCell::new(self));
        let engine_clone = engine.clone();
        let window = engine.borrow().window.clone();

        let f: Rc<RefCell<dyn FnMut() -> Result<(), JsValue>>> =
            Rc::new(RefCell::new(move || {
                let mut eng = engine_clone.borrow_mut();
                let now = eng.window.performance().unwrap().now();
                let delta = now - eng.last_frame_time;
                eng.last_frame_time = now;

                let mouse_pressed = eng.input_handler.is_mouse_button_pressed(0)
                    || eng.input_handler.is_mouse_button_pressed(1)
                    || eng.input_handler.is_mouse_button_pressed(2);

                if !mouse_pressed {
                    eng.update(delta);
                    eng.render();
                    if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                        if let Some(el) = doc.get_element_by_id("hit-indices") {
                            el.set_inner_html("None");
                        }
                    }
                } else {
                    let pos = eng.input_handler.get_mouse_position();
                    let hits = eng.hit_indices(pos.x, pos.y);
                    let hits_str = if hits.is_empty() {
                        "None".to_string()
                    } else {
                        hits.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(", ")
                    };
                    if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                        if let Some(el) = doc.get_element_by_id("hit-indices") {
                            el.set_inner_html(&hits_str);
                        }
                    }
                }

                Ok(())
            }));

        animation_frame::request_recursive(window, f)
    }

    fn update(&mut self, delta_time: f64) {
        for obj in &mut self.objects {
            obj.update(delta_time);
        }
    }

    pub fn render(&self) {
        // 1) Pick a background color
        let bg_color: JsValue = JsValue::from_str("#6C5B7B");

        // 2) Get the current window size
        let (width, height) = Rust2DEngine::get_window_inner_size(&self.window);

        // 3) Clear the entire canvas
        self.context.set_fill_style(&bg_color);
        self.context
            .fill_rect(0.0, 0.0, width as f64, height as f64);

        // 4) Draw every object
        for obj in &self.objects {
            obj.render(&self.context);
        }
    }

    #[wasm_bindgen]
    pub fn add_object(&mut self, keyframes: Vec<squre_object::Keyframe>, size: f64, color: &str) {
        let obj = squre_object::SquareObject::new(keyframes, size, color);
        self.objects.push(obj);
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
        let mut hits = Vec::new();
        for obj in &self.objects {
            let px = obj.current_x();
            let py = obj.current_y();
            let s  = obj.get_size();
            if x >= px && x <= px + s && y >= py && y <= py + s {
                hits.push(obj.index());
            }
        }
        hits
    }
}
