use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use wasm_bindgen_futures::JsFuture;
use web_sys::window;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, Window};
use std::{cell::RefCell, rc::Rc};
use std::sync::Arc;

use crate::animation_frame;
use crate::keyframe::Keyframe;
use crate::keyframe_database::KeyframeDatabase;
use crate::squre_object;
use crate::input;
use crate::squre_object::SquareObject;

use std::collections::VecDeque;

enum EngineTask {
    FetchData,
    UpdateAndRender(f64),
}

#[wasm_bindgen]
pub struct Rust2DEngine {
    window: Rc<Window>,
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
                // convert your `idb::Error` into a JsValue
                JsValue::from_str(&format!("KeyframeDatabase init failed: {}", e))
            })?;
        let task_queue = Rc::new(RefCell::new(VecDeque::new()));
        Ok(Rust2DEngine {
            window: Rc::new(window),
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

        {
            let engine_clone = engine.clone();
            engine_clone.borrow_mut().fetch_data().await?;
        }

        // fetch 루프
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

        // render 루프
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

        // 루프 실행 시작
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
                                    hits.iter()
                                        .map(|i| i.to_string())
                                        .collect::<Vec<_>>()
                                        .join(", ")
                                };
                                if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                                    if let Some(el) = doc.get_element_by_id("hit-indices") {
                                        el.set_inner_html(&hits_str);
                                    }
                                }
                            }
                        }
                    }
                }

                // 작은 delay로 CPU 과점 방지
                gloo_timers::future::TimeoutFuture::new(1).await;
            }
        });
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
        // 1) Pick a background color
        let bg_color: JsValue = JsValue::from_str("#6C5B7B");

        // 2) Get the current window size
        let (width, height) = Rust2DEngine::get_window_inner_size(&self.window);

        // 3) Clear the entire canvas
        self.context.set_fill_style(&bg_color);
        self.context
            .fill_rect(0.0, 0.0, width as f64, height as f64);

        // 4) Draw every object
        let objs = self.objects.get_mut();
        for obj in objs.iter_mut() {
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
                let px = obj.current_x();
                let py = obj.current_y();
                let s  = obj.get_size();
                if x >= px && x <= px + s && y >= py && y <= py + s {
                    Some(obj.index())
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
        // 캔버스 크기 가져오기
        let (width, height) = Rust2DEngine::get_window_inner_size(&self.window);
        let width_f64 = width as f64;
        let height_f64 = height as f64;

        // 진행 상황을 JavaScript로 보내기 위한 콜백 설정
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let loading_el = document.get_element_by_id("loading").unwrap();

        // 난수 생성기
        let rng = js_sys::Math::random;

        for idx in 0..total_objects {
            // 애니메이션 프레임 동기화
            //if idx == 0 || idx % 10 == 0 
            {
                let promise = js_sys::Promise::new(&mut |resolve, _reject| {
                    self.window
                        .request_animation_frame(&resolve)
                        .unwrap();
                });
                JsFuture::from(promise).await?;
            }

            // 진행 상황 업데이트
            let progress_ratio = (idx + 1) as f64 / total_objects as f64;
            let percentage = (progress_ratio * 100.0).floor();

            let progress_text = format!(
                "Creating objects: {} / {} ({}%)",
                idx + 1,
                total_objects,
                percentage
            );

            // 로딩 텍스트 업데이트
            loading_el.set_inner_html(&progress_text);

            // 콘솔 출력
            //web_sys::console::log_1(&progress_text.into());

            // 랜덤 색상 생성
            let color = format!("#{:06x}", (rng() * 0xFFFFFF as f64).floor() as u32);

            // 키프레임 생성
            let mut keyframes = Vec::new();
            let mut t = 0.0;

            let x0 = rng() * (width_f64 - size);
            let y0 = rng() * (height_f64 - size);
            // web_sys::console::log_1(
            //     &format!("Keyframe: time = {:.2}, x = {:.2}, y = {:.2}", t, x0, y0).into(),
            // );
            keyframes.push(Keyframe::new(t, x0, y0));

            for _ in 1..frames_per_object {
                t += rng() * 1000.0;
                let x = rng() * (width_f64 - size);
                let y = rng() * (height_f64 - size);

                // let log_msg = format!("Keyframe: time = {:.2}, x = {:.2}, y = {:.2}", t, x, y);
                // web_sys::console::log_1(&log_msg.into());

                keyframes.push(Keyframe::new(t, x, y));
            }

            // 오브젝트 추가
            self.objects
                .borrow_mut()
                .push(SquareObject::new(Arc::clone(&self.keyframe_db), keyframes, size, &color).await);
        }

        // 🎯 Preprocessing 메시지 출력
        loading_el.set_inner_html("Preprocessing...");
        //web_sys::console::log_1(&"Preprocessing...".into());

        // 📦 fetch_data 호출 (모든 오브젝트에 대해)
        let engine = Rc::new(RefCell::new(self));
        {
            let engine_clone = engine.clone();
            engine_clone.borrow_mut().fetch_data().await?;
        }

        Ok(())
    }
}
