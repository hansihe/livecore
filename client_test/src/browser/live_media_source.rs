use futures_signals::signal::Mutable;
use std::sync::Arc;
use wasm_bindgen::prelude::*;

#[derive(Debug, Copy, Clone)]
pub enum ReadyState {
    Closed,
    Open,
    Ended,
}

struct MediaSourceWrapper {
    inner: web_sys::MediaSource,
    state: Arc<Mutable<ReadyState>>,
}
impl MediaSourceWrapper {
    pub fn new() -> Self {
        let state = Arc::new(Mutable::new(ReadyState::Closed));

        let on_open = Closure::wrap(Box::new(move || {
            log::info!("yay open");
        }) as Box<dyn FnMut()>);
        let on_ended = Closure::wrap(Box::new(move || {
            log::info!("yay ended");
        }) as Box<dyn FnMut()>);
        let on_close = Closure::wrap(Box::new(move || {
            log::info!("yay close");
        }) as Box<dyn FnMut()>);

        let media_source = web_sys::MediaSource::new().unwrap();

        MediaSourceWrapper {
            inner: media_source,
            state,
        }
    }
}

pub struct LiveMediaSource {}

impl LiveMediaSource {
    pub fn new() -> Self {
        let media_source = web_sys::MediaSource::new();

        todo!()
    }
}
