use wasm_bindgen::prelude::*;
use web_sys::{MouseEvent, HtmlCanvasElement};
use std::cell::RefCell;
use std::rc::Rc;

pub struct InputHandler {
    mouse_position: Rc<RefCell<crate::math::Vector2>>,
    mouse_buttons: Rc<RefCell<Vec<bool>>>,
}

impl InputHandler {
    pub fn new(canvas: &HtmlCanvasElement) -> Result<Self, JsValue> {
        let mouse_position = Rc::new(RefCell::new(crate::math::Vector2::new(0.0, 0.0)));
        let mouse_buttons = Rc::new(RefCell::new(vec![false, false, false]));
        
        {
            let mouse_position_clone = Rc::clone(&mouse_position);
            
            let mousemove_callback = Closure::wrap(Box::new(move |event: MouseEvent| {
                // Get canvas rect using canvas.getBoundingClientRect()
                let target = event.target().unwrap();
                let canvas = target.dyn_ref::<HtmlCanvasElement>().unwrap();
                
                let rect = canvas.get_bounding_client_rect();
                
                let x = event.client_x() as f64 - rect.left();
                let y = event.client_y() as f64 - rect.top();
                
                *mouse_position_clone.borrow_mut() = crate::math::Vector2::new(x, y);
            }) as Box<dyn FnMut(_)>);
            
            canvas.add_event_listener_with_callback(
                "mousemove",
                mousemove_callback.as_ref().unchecked_ref(),
            )?;
            mousemove_callback.forget();
            
            let buttons = Rc::clone(&mouse_buttons);
            let mousedown_callback = Closure::wrap(Box::new(move |event: MouseEvent| {
                let button = event.button() as usize;
                if button < 3 {
                    buttons.borrow_mut()[button] = true;
                }
            }) as Box<dyn FnMut(_)>);
            
            canvas.add_event_listener_with_callback(
                "mousedown",
                mousedown_callback.as_ref().unchecked_ref(),
            )?;
            mousedown_callback.forget();
            
            let buttons = Rc::clone(&mouse_buttons);
            let mouseup_callback = Closure::wrap(Box::new(move |event: MouseEvent| {
                let button = event.button() as usize;
                if button < 3 {
                    buttons.borrow_mut()[button] = false;
                }
            }) as Box<dyn FnMut(_)>);
            
            canvas.add_event_listener_with_callback(
                "mouseup",
                mouseup_callback.as_ref().unchecked_ref(),
            )?;
            mouseup_callback.forget();
        }
        
        Ok(InputHandler {
            mouse_position,
            mouse_buttons,
        })
    }
    
    pub fn get_mouse_position(&self) -> crate::math::Vector2 {
        let position = self.mouse_position.borrow();
        crate::math::Vector2::new(position.x, position.y)
    }
    
    pub fn is_mouse_button_pressed(&self, button: usize) -> bool {
        if button < 3 {
            self.mouse_buttons.borrow()[button]
        } else {
            false
        }
    }
}