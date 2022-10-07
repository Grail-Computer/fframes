#[cfg(target_arch = "wasm32")]
pub mod log {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = console)]
        pub fn log(s: &str);
    }

    #[macro_export]
    macro_rules! log {
    ( $( $t:tt )* ) => {

        fframes::log(&format_args!($($t)*).to_string())
    }
}
}

#[cfg(not(target_arch = "wasm32"))]
pub mod log {
    #[macro_export]
    macro_rules! log {
    ( $( $t:tt )* ) => {
      println!( $( $t )* );
    }
    }
}
