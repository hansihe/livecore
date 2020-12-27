use std::error::Error;
use std::sync::{Arc, Mutex};

use futures::future;
use futures::stream::StreamExt;
use futures_signals::signal::Mutable;

use super::sleep;
use crate::fabric_client::FabricConnState;

use wasm_bindgen::convert::IntoWasmAbi;
use wasm_bindgen::JsValue;

use tokio::sync::mpsc::{Sender, channel};

pub struct FabricClient {
    shared: Arc<FabricClientShared>,
}

struct FabricClientShared {
    state: Mutable<FabricConnState>,
    cancel: Mutable<bool>,
    //tmp_callback: Mutex<Option<js_sys::Function>>,
}

impl FabricClient {
    pub fn new(url: String) -> Self {
        let shared = Arc::new(FabricClientShared {
            state: Mutable::new(FabricConnState::Connecting),
            cancel: Mutable::new(false),
            //tmp_callback: Mutex::new(None),
        });

        let shared_f = shared.clone();
        wasm_bindgen_futures::spawn_local(async move {
            loop {
                log::info!("yay websocket loop");
                // TODO backoff logic
                match do_fabric_loop(&shared_f, &url).await {
                    Ok(()) => {
                        log::error!("do_fabric_loop should never exit successfully");
                        panic!();
                    }
                    Err(err) => {
                        log::error!("fabric loop error: {}", err);
                        sleep(1000).await;
                    }
                }
            }
        });

        FabricClient { shared }
    }

    //pub fn set_tmp_callback(&self, cb: js_sys::Function) {
    //    (*self.shared.tmp_callback.lock().unwrap()) = Some(cb);
    //}
}

async fn do_fabric_loop(shared: &Arc<FabricClientShared>, url: &str) -> Result<(), Box<dyn Error>> {
    // TODO Error handling
    let mut ws = super::websocket::WebSocket::connect(url.to_string()).unwrap();
    ws.wait_open().await?;

    let messages = ws
        .messages()
        .unwrap()
        .take_until(ws.wait_close())
        .filter_map(|data| async move {
            let array = js_sys::Uint8Array::new(&data);

            // TODO: less copy, use blob?
            let vec = array.to_vec();

            match livecore_protocol::deserialize(&vec) {
                Ok(msg) => Some(msg),
                Err(err) => {
                    log::error!("failed to deserialize message!! {}", err);
                    None
                }
            }
        });

    messages
        .for_each(|msg| {
            match msg {
                livecore_protocol::Message::StreamData { data } => {
                    //let cb_guard = shared.tmp_callback.lock().unwrap();
                    //let cb = cb_guard.as_ref().unwrap();

                    let data_slice = &data[..];
                    let value: js_sys::Uint8Array = data_slice.into();

                    let this = JsValue::null();
                    log::info!("before callback call");
                    match cb.call1(&this, &value.into()) {
                        Ok(val) => log::info!("ret: {:?}", val),
                        Err(val) => log::info!("fail: {:?}", val),
                    }
                    log::info!("after callback call");
                }
            }
            log::info!("yay msg");
            future::ready(())
        })
        .await;

    ws.wait_close().await;

    log::info!("yay");

    Ok(())
}
