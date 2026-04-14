use wasm_bindgen::prelude::*;
use web_sys::{window, Event, HtmlFormElement};

pub fn init() {
    let doc = match window().and_then(|w| w.document()) {
        Some(d) => d,
        None => return,
    };

    let forms = doc.query_selector_all("form[data-confirm]").unwrap();

    for i in 0..forms.length() {
        if let Some(node) = forms.item(i) {
            let form: HtmlFormElement = match node.dyn_into() {
                Ok(f) => f,
                Err(_) => continue,
            };

            let message: String = match form.get_attribute("data-confirm") {
                Some(msg) => msg,
                None => continue,
            };

            let closure = Closure::<dyn Fn(Event)>::new(move |event: Event| {
                let result = window().and_then(|w| {
                    w.confirm_with_message(&message).ok()
                });
                if !result.unwrap_or(false) {
                    event.prevent_default();
                }
            });

            let _ = form.add_event_listener_with_callback("submit", closure.as_ref().unchecked_ref());
            closure.forget();
        }
    }
}