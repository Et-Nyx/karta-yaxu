use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use web_sys::{window, HtmlElement, HtmlImageElement};

pub fn init() {
    let doc = match window().and_then(|w| w.document()) {
        Some(d) => d,
        None => return,
    };

    let shiny_btn = match doc.get_element_by_id("shiny-toggle") {
        Some(el) => el,
        None => return,
    };
    let shiny_btn: HtmlElement = shiny_btn.dyn_into().unwrap_or_else(|e| e.unchecked_into());

    let sprite_el = match doc.get_element_by_id("pokemon-sprite") {
        Some(el) => el,
        None => return,
    };
    let sprite: HtmlImageElement = match sprite_el.dyn_into() {
        Ok(el) => el,
        Err(_) => return,
    };

    let normal_url: String = match sprite.get_attribute("data-normal") {
        Some(url) => url,
        None => return,
    };

    let sprite_file: String = match sprite.get_attribute("data-sprite") {
        Some(f) => f,
        None => return,
    };

    let shiny_file = sprite_file.replace(".png", "_shiny.png");
    let shiny_url = format!("/static/sprites/shiny/{}?v=1", shiny_file);

    let state = Rc::new(RefCell::new(false));
    let state_clone = state.clone();
    let sprite_clone = sprite;
    let shiny_btn_clone = shiny_btn.clone();
    let normal_url_clone = normal_url.clone();

    let closure = Closure::<dyn Fn()>::new(move || {
        let mut is_shiny = state_clone.borrow_mut();
        *is_shiny = !*is_shiny;
        if *is_shiny {
            sprite_clone.set_src(&shiny_url);
            let _ = shiny_btn_clone.style().set_property("border", "2px solid var(--border)");
            let _ = shiny_btn_clone.style().set_property("background-color", "var(--gb-light)");
        } else {
            sprite_clone.set_src(&normal_url_clone);
            let _ = shiny_btn_clone.style().set_property("border", "2px solid transparent");
            let _ = shiny_btn_clone.style().set_property("background-color", "transparent");
        }
    });

    let _ = shiny_btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref());
    closure.forget();
}