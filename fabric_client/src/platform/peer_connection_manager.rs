use futures::sink::{Sink, SinkExt};
use futures::stream::Stream;

use std::future::Future;
use std::pin::Pin;

use anyhow::Result;

use livecore_protocol as proto;
use proto::Uuid;

pub trait PeerConnectionManager {
    fn start_connect_peer(
        &self,
        peer_tunnel: PeerTunnel,
        conn_type: proto::Connector,
        self_nonce: Uuid,
        peer_nonce: Uuid,
    ) -> Pin<Box<dyn Future<Output = Result<PeerConnection>> + Send>>;
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
