use wasm_bindgen::prelude::*;
use web_sys::{window, HtmlElement};

fn el_to_html(el: web_sys::Element) -> HtmlElement {
    el.dyn_into().unwrap_or_else(|e| e.unchecked_into())
}

const TRANSLATIONS: &[(&str, &str)] = &[
    ("pokemon", "Pokémon"),
    ("mechanics", "Mecânicas"),
    ("abilities", "Habilidades"),
    ("types", "Tipos"),
    ("moves", "Moves"),
    ("natures", "Natures"),
    ("status", "Status"),
    ("wiki", "Wiki"),
    ("editor", "Editor"),
    ("trainer", "Trainer"),
];

fn translate_segment(segment: &str) -> String {
    let lower = segment.to_lowercase();
    for (key, value) in TRANSLATIONS {
        if lower == *key {
            return (*value).to_string();
        }
    }
    let mut display = segment.replace('-', " ");
    if let Some(c) = display.chars().next() {
        display = c.to_uppercase().collect::<String>() + &display[1..];
    }
    display
}

pub fn init() {
    let doc = match window().and_then(|w| w.document()) {
        Some(d) => d,
        None => return,
    };

    let container = match doc.get_element_by_id("breadcrumbs") {
        Some(el) => el_to_html(el),
        None => return,
    };

    let path = match window().and_then(|w| w.location().pathname().ok()) {
        Some(p) => p,
        None => return,
    };

    if path == "/" || path.is_empty() {
        let _ = container.style().set_property("display", "none");
        return;
    }

    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if segments.is_empty() {
        let _ = container.style().set_property("display", "none");
        return;
    }

    let mut current_url = String::new();

    for (i, segment) in segments.iter().enumerate() {
        current_url.push('/');
        current_url.push_str(segment);

        let separator = doc.create_element("span").unwrap();
        separator.set_text_content(Some(">"));
        let _ = separator.set_attribute("style", "opacity: 0.5; margin: 0 4px;");
        let _ = container.append_child(&separator);

        let display_name = translate_segment(segment);
        let is_last = i == segments.len() - 1;

        if is_last {
            let span = doc.create_element("span").unwrap();
            span.set_text_content(Some(&display_name));
            let _ = span.set_attribute("style", "text-transform: uppercase");
            let _ = container.append_child(&span);
        } else {
            let a = doc.create_element("a").unwrap();
            let _ = a.set_attribute("href", &current_url);
            let _ = a.set_attribute("style", "text-decoration: none; text-transform: uppercase");
            a.set_text_content(Some(&display_name));
            let _ = container.append_child(&a);
        }
    }
}