use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, Window};
use std::{cell::RefCell, rc::Rc};

use crate::animation_frame;
use crate::keyframe::Keyframe;
use crate::keyframe_database::KeyframeDatabase;
use crate::squre_object;
use crate::input;
use crate::squre_object::SquareObject;

#[wasm_bindgen]
pub struct Rust2DEngine {
    window: Rc<Window>,
    context: CanvasRenderingContext2d,
    last_frame_time: f64,
    objects: RefCell<Vec<squre_object::SquareObject>>,
    input_handler: input::InputHandler,
    keyframe_db: Rc<KeyframeDatabase>,
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
        //let keyframe_db = Rc::new(db);

        Ok(Rust2DEngine {
            window: Rc::new(window),
            context,
            last_frame_time,
            objects: RefCell::new(Vec::new()),
            input_handler,
            keyframe_db: keyframe_db,
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

    async fn add_object(
        &self,
        keyframes: Vec<Keyframe>,
        size: f64,
        color: &str,
    ) -> Result<(), JsValue> {
        // let obj = squre_object::SquareObject::new(
        //     Rc::clone(&self.keyframe_db),
        //     keyframes,
        //     size,
        //     color,
        // )
        // .await;
        // self.objects.push(obj);
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
        &self, 
        total_objects: u32, 
        frames_per_object: u32, 
        size: f64
    ) -> Result<(), JsValue> {
        // 캔버스 크기 가져오기
        let (width, height) = Rust2DEngine::get_window_inner_size(&self.window);
        let width_f64 = width as f64;
        let height_f64 = height as f64;
        
        // 진행 상황을 JavaScript로 보내기 위한 콜백 설정
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let loading_el = document.get_element_by_id("loading").unwrap();
        
        //난수 생성기
        let rng = js_sys::Math::random;
        
        for idx in 0..total_objects {
            // 진행 상황 업데이트
            let progress_ratio = (idx + 1) as f64 / total_objects as f64;
            let percentage = (progress_ratio * 100.0).floor();
            
            let progress_text = format!(
                "Creating objects: {} / {} ({}%)", 
                idx + 1, total_objects, percentage
            );
            
            // 로딩 텍스트 업데이트
            loading_el.set_text_content(Some(&progress_text));
            
            // 콘솔에 진행 상황 출력
            web_sys::console::log_1(&progress_text.into());
            
            // 랜덤 색상 생성
            let color = format!(
                "#{:06x}", 
                (rng() * 0xFFFFFF as f64).floor() as u32
            );
            
            // 일반 객체 생성 - 모든 키프레임 미리 로드
            let mut keyframes = Vec::new();
            let mut t = 0.0;
            
            for _ in 0..frames_per_object {
                t += rng() * 5000.0;
                let x = rng() * (width_f64 - size);
                let y = rng() * (height_f64 - size);
                
                keyframes.push(Keyframe::new(t, x, y));
            }
            
            //self.add_object(keyframes, size, &color).await?;
            self.objects.borrow_mut()
               .push(SquareObject::new(Rc::clone(&self.keyframe_db), keyframes, size, &color).await);
        }
        
        Ok(())
    }

    // #[wasm_bindgen]
    // pub async fn generate_objects2(
    //     &self,
    //     total_objects: u32,
    //     frames_per_object: u32,
    //     size: f64,
    // ) -> Result<(), JsValue> {
    //     // 1) Wasm 스레드 풀 초기화
    //     let _ = init_thread_pool(total_objects as usize);

    //     // 2) 캔버스 크기 가져오기
    //     let (w, h) = Rust2DEngine::get_window_inner_size(&self.window);
    //     let width_f  = w as f64;
    //     let height_f = h as f64;

    //     // 3) js_sys::Math::random을 함수 포인터로 받기
    //     let rng: fn() -> f64 = js_sys::Math::random;

    //     // 4) Rayon으로 병렬 키프레임·색상 생성
    //     let params: Vec<(Vec<Keyframe>, f64, String)> = 
    //         (0..total_objects)
    //         .into_par_iter()
    //         .map(|_| {
    //             // 키프레임 생성
    //             let mut keyframes = Vec::with_capacity(frames_per_object as usize);
    //             let mut t = 0.0;
    //             for _ in 0..frames_per_object {
    //                 t += rng() * 5000.0;
    //                 let x = rng() * (width_f - size);
    //                 let y = rng() * (height_f - size);
    //                 keyframes.push(Keyframe::new(t, x, y));
    //             }
    //             // 랜덤 색상
    //             let color = format!(
    //                 "#{:06x}",
    //                 (rng() * 16777215.0).floor() as u32
    //             );
    //             (keyframes, size, color)
    //         })
    //         .collect();

    //     // 5) 메인 스레드에서 DOM 요소 가져오기
    //     let doc        = web_sys::window().unwrap().document().unwrap();
    //     let loading_el = doc.get_element_by_id("loading").unwrap();

    //     // 6) 순차 업데이트 및 비동기 객체 생성
    //     for (idx, (keyframes, size, color)) in params.into_iter().enumerate() {
    //         let done = ((idx as f64 + 1.0) / total_objects as f64 * 100.0).floor() as u32;
    //         let msg = format!(
    //             "Creating objects: {} / {} ({}%)",
    //             idx + 1,
    //             total_objects,
    //             done
    //         );
    //         loading_el.set_text_content(Some(&msg));
    //         web_sys::console::log_1(&msg.into());

    //         // 실제 엔진 객체 생성 호출
    //         self.add_object(keyframes, size, &color).await?;
    //     }

    //     Ok(())
    // }
}
