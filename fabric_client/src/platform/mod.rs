use futures::sink::{Sink, SinkExt};
use futures::stream::Stream;
use serde_json::Value as JsonValue;

use std::future::Future;
use std::pin::Pin;

#[cfg(target_arch = "wasm32")]
mod browser;
#[cfg(target_arch = "wasm32")]
use browser::*;

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
use native::*;

pub trait PeerConnectionManager {
    fn start_connect_peer(
        &self,
        peer_tunnel: PeerTunnel,
        config: JsonValue,
    ) -> Pin<Box<dyn Future<Output = PeerConnection> + Send>>;
}

pub struct PeerTunnel {
    sink: Box<dyn Sink<Vec<u8>, Error = ()> + Send>,
    source: Box<dyn Stream<Item = Vec<u8>> + Send>,
}
impl PeerTunnel {
    pub fn new_dummy() -> Self {
        Self {
            sink: Box::new(futures::sink::drain().sink_map_err(|_| panic!())),
            source: Box::new(futures::stream::empty()),
        }
    }
}

pub enum PeerConnectionError {
    Wat,
}

pub struct PeerConnection {
    //meta: Box<dyn PeerConnectionMeta>,
    pub sink: Pin<Box<dyn Sink<Vec<u8>, Error = PeerConnectionError> + Send>>,
    pub source: Pin<Box<dyn Stream<Item = Result<Vec<u8>, PeerConnectionError>> + Send>>,
}

pub struct PeerData {
    pubkey: Vec<u8>,
}

trait PeerConnectionMeta {}
