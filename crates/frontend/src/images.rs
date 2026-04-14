use wasm_bindgen::prelude::*;
use web_sys::{window, HtmlImageElement};

pub fn init() {
    let doc = match window().and_then(|w| w.document()) {
        Some(d) => d,
        None => return,
    };

    let images = doc.query_selector_all("img[data-fallback]").unwrap();

    for i in 0..images.length() {
        if let Some(node) = images.item(i) {
            let img: HtmlImageElement = match node.dyn_into() {
                Ok(el) => el,
                Err(_) => continue,
            };

            let fallback: String = match img.get_attribute("data-fallback") {
                Some(f) => f,
                None => continue,
            };

            if fallback == "hide" {
                let img_ref = img.clone();
                let closure = Closure::<dyn Fn()>::new(move || {
                    let _ = img_ref.style().set_property("display", "none");
                });
                let _ = img.add_event_listener_with_callback("error", closure.as_ref().unchecked_ref());
                closure.forget();
            }
        }
    }
}