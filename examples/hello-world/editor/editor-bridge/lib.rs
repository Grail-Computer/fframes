use fframes_editor_controller::prelude::*;
use hello_world_example::{HelloWorldMedia, HelloWorldVideo};

setup_wasm_bridge!(HelloWorldVideo, HelloWorldMedia);

lazy_static! {
    static ref MEDIA: HelloWorldMedia =
        HelloWorldMedia::prepare().expect("Failed static media processing for wasm bridge");
    static ref VIDEO: HelloWorldVideo<'static> = HelloWorldVideo {
        media: &MEDIA,
        slug: "World",
    };
}

#[wasm_bindgen]
pub fn create_wasm_bridge() -> WasmBridge {
    console_error_panic_hook::set_once();

    WasmBridge::new(&VIDEO, &MEDIA)
}
