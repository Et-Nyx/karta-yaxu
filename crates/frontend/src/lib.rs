pub mod breadcrumbs;
pub mod filters;
pub mod forms;
pub mod images;
pub mod search;
pub mod shiny;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
    breadcrumbs::init();
    search::init();
    shiny::init();
    filters::init();
    forms::init();
    images::init();
}