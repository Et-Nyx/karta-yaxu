use wasm_bindgen::prelude::*;
use web_sys::{window, HtmlElement, HtmlInputElement, KeyboardEvent};

fn node_to_html(node: web_sys::Node) -> HtmlElement {
    node.unchecked_into()
}

pub fn init() {
    let doc = match window().and_then(|w| w.document()) {
        Some(d) => d,
        None => return,
    };

    let search_bar = match doc.get_element_by_id("search-bar") {
        Some(el) => el,
        None => return,
    };
    let input: HtmlInputElement = match search_bar.dyn_into() {
        Ok(el) => el,
        Err(_) => return,
    };

    let has_searchable = doc.query_selector_all("[data-search-name]").unwrap().length() > 0;

    let input_ref = input.clone();

    let input_closure = Closure::<dyn Fn()>::new(move || {
        let query = input_ref.value();
        if has_searchable {
            filter_cards(&query.to_lowercase());
        }
    });
    let _ = input.add_event_listener_with_callback("input", input_closure.as_ref().unchecked_ref());
    input_closure.forget();

    let body = match doc.body() {
        Some(b) => b,
        None => return,
    };

    let global_closure = Closure::<dyn Fn(KeyboardEvent)>::new(move |event: KeyboardEvent| {
        let key = event.key();
        let target = event.target();
        let is_text_input = target.map_or(false, |t| {
            let el: HtmlElement = t.unchecked_into();
            let tag = el.tag_name().to_lowercase();
            tag == "input" || tag == "textarea" || tag == "select"
        });

        if key == "/" && !is_text_input {
            event.prevent_default();
            let d = window().and_then(|w| w.document());
            if let Some(d) = d {
                if let Some(el) = d.get_element_by_id("search-bar") {
                    let search_input: HtmlInputElement = el.unchecked_into();
                    let _ = search_input.focus();
                }
            }
        }

        if key == "Escape" {
            let d = window().and_then(|w| w.document());
            if let Some(d) = d {
                if let Some(el) = d.get_element_by_id("search-bar") {
                    let search_input: HtmlInputElement = el.unchecked_into();
                    search_input.set_value("");
                    let _ = search_input.blur();
                }
            }
            filter_cards("");
        }
    });
    let _ = body.add_event_listener_with_callback("keydown", global_closure.as_ref().unchecked_ref());
    global_closure.forget();

    if let Some(toggle_btn) = doc.get_element_by_id("facet-toggle") {
        let toggle_closure = Closure::<dyn Fn()>::new(move || {
            let d = window().and_then(|w| w.document());
            if let Some(d) = d {
                if let Some(facets_el) = d.get_element_by_id("search-facets") {
                    let _ = facets_el.class_list().toggle("search-facets--open");
                }
            }
        });
        let _ = toggle_btn.add_event_listener_with_callback("click", toggle_closure.as_ref().unchecked_ref());
        toggle_closure.forget();
    }
}

fn filter_cards(query: &str) {
    let doc = match window().and_then(|w| w.document()) {
        Some(d) => d,
        None => return,
    };

    let searchable = match doc.query_selector_all("[data-search-name]") {
        Ok(s) => s,
        Err(_) => return,
    };

    for i in 0..searchable.length() {
        if let Some(node) = searchable.item(i) {
            let el = node_to_html(node);
            let name = match el.get_attribute("data-search-name") {
                Some(n) => n.to_lowercase(),
                None => continue,
            };
            if query.is_empty() || name.contains(query) {
                let _ = el.style().set_property("display", "");
            } else {
                let _ = el.style().set_property("display", "none");
            }
        }
    }
}