use std::{rc::Rc, cell::RefCell};
use wasm_bindgen::{prelude::*, JsCast};
use web_sys::Window;

pub fn request_recursive(
    window: Rc<Window>,
    callback: Rc<RefCell<dyn FnMut() -> Result<(), JsValue>>>,
) -> Result<(), JsValue> {
    fn request_frame(
        window: &Window,
        callback: &Rc<RefCell<dyn FnMut() -> Result<(), JsValue>>>,
    ) -> Result<(), JsValue> {
        let window_clone = window.clone();
        let callback_clone = callback.clone();
        
        let closure = Closure::once_into_js(Box::new(move || {
            callback_clone.borrow_mut()().unwrap();
            
            // Schedule the next frame
            request_frame(&window_clone, &callback_clone).unwrap();
        }) as Box<dyn FnOnce()>);
        
        // Start the animation frame
        window.request_animation_frame(closure.unchecked_ref())?;
        
        Ok(())
    }
    
    // Start the recursive loop
    request_frame(&window, &callback)
}
