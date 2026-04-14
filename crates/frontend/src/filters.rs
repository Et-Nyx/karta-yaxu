use wasm_bindgen::prelude::*;
use web_sys::{window, HtmlElement};

fn node_to_html(node: web_sys::Node) -> HtmlElement {
    node.unchecked_into()
}

pub fn init() {
    let doc = match window().and_then(|w| w.document()) {
        Some(d) => d,
        None => return,
    };

    let filters = doc.query_selector_all(".method-filter").unwrap();
    let moves = doc.query_selector_all(".learnset-move").unwrap();

    if filters.length() == 0 || moves.length() == 0 {
        return;
    }

    let filter_elements: Vec<HtmlElement> = (0..filters.length())
        .filter_map(|i| filters.item(i))
        .map(node_to_html)
        .collect();

    let move_elements: Vec<HtmlElement> = (0..moves.length())
        .filter_map(|i| moves.item(i))
        .map(node_to_html)
        .collect();

    for idx in 0..filter_elements.len() {
        let filter_btns = filter_elements.clone();
        let move_els = move_elements.clone();

        let closure = Closure::<dyn Fn()>::new(move || {
            for btn in &filter_btns {
                let _ = btn.style().set_property("background-color", "var(--primary)");
                let _ = btn.style().set_property("color", "var(--primary-foreground)");
            }

            let active = &filter_btns[idx];
            let _ = active.style().set_property("background-color", "var(--gb-light)");
            let _ = active.style().set_property("color", "var(--gb-darkest)");

            let method: String = active.get_attribute("data-filter").unwrap_or_else(|| "all".to_string());

            for m in &move_els {
                let m_method: String = m.get_attribute("data-method").unwrap_or_default();
                if method == "all" || m_method == method {
                    let _ = m.style().set_property("display", "flex");
                } else {
                    let _ = m.style().set_property("display", "none");
                }
            }
        });

        let _ = filter_elements[idx].add_event_listener_with_callback("click", closure.as_ref().unchecked_ref());
        closure.forget();
    }
}