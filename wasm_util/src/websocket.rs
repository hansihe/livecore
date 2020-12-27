use std::error::Error;
use std::sync::Arc;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use futures::channel::mpsc::{unbounded, UnboundedReceiver};
use futures::stream::Stream;

use tokio::sync::watch;

use web_sys::{MessageEvent, WebSocket as NWS};

#[derive(Copy, Clone)]
pub enum WSState {
    Connecting,
    Connected,
    Closing,
    Closed,
}

#[derive(Debug)]
pub struct WSError {}
impl std::fmt::Display for WSError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "websocket error")
    }
}
impl Error for WSError {}

pub struct WebSocket {
    #[allow(dead_code)]
    inner: NWS,

    state: watch::Receiver<WSState>,

    messages: Option<UnboundedReceiver<js_sys::ArrayBuffer>>,

    #[allow(dead_code)]
    on_open_closure: Closure<dyn FnMut()>,
    #[allow(dead_code)]
    on_error_closure: Closure<dyn FnMut()>,
    #[allow(dead_code)]
    on_close_closure: Closure<dyn FnMut()>,
    #[allow(dead_code)]
    on_message_closure: Closure<dyn FnMut(MessageEvent)>,
}

impl WebSocket {
    fn wrap(create: impl FnOnce() -> Result<NWS, JsValue>) -> Result<Self, JsValue> {
        //let state = Arc::new(Mutable::new(WSState::Connecting));

        let (state_sender, state_receiver) = watch::channel(WSState::Connecting);
        let state_sender = Arc::new(state_sender);

        let (msg_sender, msg_receiver) = unbounded();

        let on_open = {
            let state_sender = state_sender.clone();
            Closure::wrap(Box::new(move || {
                // We don't really care if this fails, fail only happens if the
                // other end of the channel is gone.
                let _ = state_sender.send(WSState::Connected);
            }) as Box<dyn FnMut()>)
        };
        let on_error = Closure::wrap(Box::new(move || {}) as Box<dyn FnMut()>);
        let on_close = {
            let state_sender = state_sender.clone();
            Closure::wrap(Box::new(move || {
                // We don't really care if this fails, fail only happens if the
                // other end of the channel is gone.
                let _ = state_sender.send(WSState::Closed);
            }) as Box<dyn FnMut()>)
        };
        let on_message = Closure::wrap(Box::new(move |event: MessageEvent| {
            if let Ok(abuf) = event.data().dyn_into::<js_sys::ArrayBuffer>() {
                msg_sender
                    .unbounded_send(abuf)
                    .expect("unbounded send failed, receiver dropped?");
                //log::info!("message: {:?}", result);
            } else {
                panic!("unknown message type");
            }
        }) as Box<dyn FnMut(MessageEvent)>);

        let nws = create()?;

        nws.set_binary_type(web_sys::BinaryType::Arraybuffer);

        nws.set_onopen(Some(&on_open.as_ref().unchecked_ref()));
        nws.set_onerror(Some(&on_error.as_ref().unchecked_ref()));
        nws.set_onclose(Some(&on_close.as_ref().unchecked_ref()));
        nws.set_onmessage(Some(&on_message.as_ref().unchecked_ref()));

        let ws = WebSocket {
            inner: nws,
            state: state_receiver,

            messages: Some(msg_receiver),

            on_open_closure: on_open,
            on_error_closure: on_error,
            on_close_closure: on_close,
            on_message_closure: on_message,
        };

        Ok(ws)
    }

    pub fn connect(url: String) -> Result<Self, JsValue> {
        WebSocket::wrap(|| NWS::new(&url))
    }

    pub fn messages(&mut self) -> Option<impl Stream<Item = js_sys::ArrayBuffer>> {
        self.messages.take()
    }

    pub async fn wait_open(&self) -> Result<(), WSError> {
        let mut recv = self.state.clone();

        let mut state = *recv.borrow();
        loop {
            match state {
                WSState::Connecting => (),
                WSState::Connected => break Ok(()),
                WSState::Closing => break Err(WSError {}),
                WSState::Closed => break Err(WSError {}),
            }

            recv.changed().await.unwrap();
            state = *recv.borrow();
        }
    }

    pub async fn wait_close(&self) {
        let mut recv = self.state.clone();

        let mut state = *recv.borrow();
        loop {
            match state {
                WSState::Closing => break,
                WSState::Closed => break,
                _ => (),
            }

            recv.changed().await.unwrap();
            state = *recv.borrow();
        }
    }
}
