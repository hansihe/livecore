use std::sync::Arc;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use tokio::sync::{watch, Notify};

#[derive(Debug, Copy, Clone)]
pub enum ReadyState {
    Closed,
    Open,
    Ended,
}

mod add_source_buffer_error {
    use snafu::Snafu;

    #[derive(Debug, Snafu)]
    pub enum AddSourceBufferError {
        #[snafu(display("empty mime type provided to add_source_buffer: {:?}", js))]
        EmptyMime { js: js_sys::TypeError },
        #[snafu(display("invalid mime type to add_source_buffer: {:?}", js))]
        UnsupportedMime { js: web_sys::DomException },
        #[snafu(display("quota exceeded on MediaSource: {:?}", js))]
        QuotaExceeded { js: web_sys::DomException },
        #[snafu(display("invalid state on MediaSource: {:?}", js))]
        InvalidState { js: web_sys::DomException },
    }
}
pub use self::add_source_buffer_error::AddSourceBufferError;

mod append_buffer_error {
    use snafu::Snafu;

    #[derive(Debug, Snafu)]
    pub enum AppendBufferError {
        InvalidState { js: web_sys::DomException },
        QuotaExceeded { js: web_sys::DomException },
    }
}
pub use self::append_buffer_error::AppendBufferError;

pub struct SourceBuffer {
    inner: web_sys::SourceBuffer,
    update_done: Arc<Notify>,

    #[allow(dead_code)]
    on_update_end_closure: Closure<dyn FnMut()>,
}
impl SourceBuffer {
    fn wrap(inner: web_sys::SourceBuffer) -> Self {
        let update_done = Arc::new(Notify::new());

        let on_update_end = {
            let update_done = update_done.clone();
            Closure::wrap(Box::new(move || {
                update_done.notify_waiters();
            }) as Box<dyn FnMut()>)
        };

        inner.set_onupdateend(Some(&on_update_end.as_ref().unchecked_ref()));

        SourceBuffer {
            inner,
            update_done,
            on_update_end_closure: on_update_end,
        }
    }

    async fn append_buffer(
        &self,
        do_append: impl Fn(&web_sys::SourceBuffer) -> Result<(), JsValue>,
    ) -> Result<(), AppendBufferError> {
        let notified = self.update_done.notified();
        match do_append(&self.inner) {
            Ok(()) => {
                notified.await;
                Ok(())
            }
            Err(err) if err.has_type::<web_sys::DomException>() => {
                let err: web_sys::DomException = err.unchecked_into();
                match err.code() {
                    web_sys::DomException::QUOTA_EXCEEDED_ERR => {
                        Err(AppendBufferError::QuotaExceeded { js: err })
                    }
                    web_sys::DomException::INVALID_STATE_ERR => {
                        Err(AppendBufferError::InvalidState { js: err })
                    }
                    _ => unreachable!(),
                }
            }
            _ => unreachable!(),
        }
    }

    pub async fn append_buffer_slice(&self, data: &[u8]) -> Result<(), AppendBufferError> {
        self.append_buffer(|sb| {
            // TODO: This is technically UB in rust, but targetting WASM only should
            // limit the impact of this at least. Should probably find some other
            // way of calling `append_buffer` which has it's argment inproperly
            // marked as mutable.
            #[allow(mutable_transmutes)]
            let transmuted: &mut [u8] = unsafe { std::mem::transmute(data) };

            sb.append_buffer_with_u8_array(transmuted)
        })
        .await
    }
}

pub struct MediaSource {
    inner: web_sys::MediaSource,
    state: watch::Receiver<ReadyState>,

    #[allow(dead_code)]
    on_open_closure: Closure<dyn FnMut()>,
    #[allow(dead_code)]
    on_ended_closure: Closure<dyn FnMut()>,
    #[allow(dead_code)]
    on_closed_closure: Closure<dyn FnMut()>,
}
impl MediaSource {
    pub fn new() -> Self {
        let (state_sender, state_receiver) = watch::channel(ReadyState::Closed);
        let state_sender = Arc::new(state_sender);

        let on_open = {
            let state_sender = state_sender.clone();
            Closure::wrap(Box::new(move || {
                let _ = state_sender.send(ReadyState::Open);
            }) as Box<dyn FnMut()>)
        };
        let on_ended = {
            let state_sender = state_sender.clone();
            Closure::wrap(Box::new(move || {
                let _ = state_sender.send(ReadyState::Ended);
            }) as Box<dyn FnMut()>)
        };
        let on_closed = {
            let state_sender = state_sender.clone();
            Closure::wrap(Box::new(move || {
                let _ = state_sender.send(ReadyState::Closed);
            }) as Box<dyn FnMut()>)
        };

        let media_source = web_sys::MediaSource::new().unwrap();
        media_source.set_onsourceopen(Some(&on_open.as_ref().unchecked_ref()));
        media_source.set_onsourceended(Some(&on_ended.as_ref().unchecked_ref()));
        media_source.set_onsourceclosed(Some(&on_closed.as_ref().unchecked_ref()));

        MediaSource {
            inner: media_source,
            state: state_receiver,

            on_open_closure: on_open,
            on_ended_closure: on_ended,
            on_closed_closure: on_closed,
        }
    }

    pub fn add_source_buffer(&self, mime_type: &str) -> Result<SourceBuffer, AddSourceBufferError> {
        match self.inner.add_source_buffer(mime_type) {
            Ok(inner) => Ok(SourceBuffer::wrap(inner)),
            Err(err) if err.has_type::<js_sys::TypeError>() => {
                Err(AddSourceBufferError::EmptyMime {
                    js: err.unchecked_into(),
                })
            }
            Err(err) if err.has_type::<web_sys::DomException>() => {
                let err: web_sys::DomException = err.unchecked_into();
                match err.code() {
                    web_sys::DomException::NOT_SUPPORTED_ERR => {
                        Err(AddSourceBufferError::UnsupportedMime { js: err })
                    }
                    web_sys::DomException::QUOTA_EXCEEDED_ERR => {
                        Err(AddSourceBufferError::QuotaExceeded { js: err })
                    }
                    web_sys::DomException::INVALID_STATE_ERR => {
                        Err(AddSourceBufferError::InvalidState { js: err })
                    }
                    _ => unreachable!(),
                }
            }
            _ => unreachable!(),
        }
    }

    pub async fn wait_source_open(&self) {
        let mut recv = self.state.clone();

        let mut state = *recv.borrow();
        loop {
            match state {
                ReadyState::Open => return,
                _ => (),
            }

            recv.changed().await.unwrap();
            state = *recv.borrow();
        }
    }

    pub fn create_url(&self) -> String {
        web_sys::Url::create_object_url_with_source(&self.inner).unwrap()
    }
}
