#![cfg(target_arch = "wasm32")]
use fframes_editor_controller::{prelude::*, setup_wasm_bridge};
use marketing_example::{MarketingMedia, MarketingVideo};

lazy_static! {
    static ref MEDIA: MarketingMedia =
        MarketingMedia::prepare().expect("Failed to create static media");
}

setup_wasm_editor!(
    MarketingVideo,
    MarketingVideo {
        media: &MEDIA,
        audio_track: "marketing.mp3"
    },
    *MEDIA
);
