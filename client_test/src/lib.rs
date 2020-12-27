use log::info;
use std::sync::{Arc, Mutex};

use futures_signals::signal::Mutable;
use wasm_bindgen::prelude::*;

use ws_stream_wasm::WsMeta;

mod utils;

mod fabric_client;

//#[cfg(target_arch = "wasm32")]
mod browser;
//#[cfg(target_arch = "wasm32")]
use browser as platform;

//#[cfg(not(target_arch = "wasm32"))]
//mod native;
//#[cfg(not(target_arch = "wasm32"))]
//use native as platform;

use platform::FabricClient;
use platform::{MediaSource, SourceBuffer};

macro_rules! console_log {
	($($t:tt)*) => {
		(web_sys::console::log_1(&format!($($t)*).into()))
	};
}

#[wasm_bindgen]
pub fn init() {
    wasm_logger::init(wasm_logger::Config::default());
}

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum FabricState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
}

struct FabricInner {
    client: FabricClient,
    ms_url: String,
}

impl FabricInner {
    pub fn new(fabric_url: String) -> Self {
        let client = FabricClient::new(fabric_url);

        let media_source = MediaSource::new();
        let ms_url = media_source.create_url();

        wasm_bindgen_futures::spawn_local(async move {
            media_source.wait_source_open().await;

            let source_buffer = media_source
                .add_source_buffer("video/mp4; codecs=\"avc3.42E01E\"")
                .unwrap();

            loop {

            }
        });

        FabricInner { client, ms_url }
    }

    pub fn get_media_source_url(&self) -> String {
        self.ms_url.clone()
    }

    pub fn tmp_set_got_segment_cb(&self, cb: js_sys::Function) {
        self.client.set_tmp_callback(cb)
    }
}

#[wasm_bindgen]
#[derive(Clone)]
pub struct Fabric(Arc<Mutex<FabricInner>>);

#[wasm_bindgen]
impl Fabric {
    #[wasm_bindgen(constructor)]
    pub fn new(fabric_url: String) -> Self {
        Fabric(Arc::new(Mutex::new(FabricInner::new(fabric_url))))
    }
    #[wasm_bindgen]
    pub fn get_media_source_url(&self) -> String {
        let inner = self.0.lock().unwrap();
        inner.get_media_source_url()
    }
    #[wasm_bindgen]
    pub fn tmp_set_got_segment_cb(&self, cb: js_sys::Function) {
        let inner = self.0.lock().unwrap();
        inner.tmp_set_got_segment_cb(cb);
    }
}
