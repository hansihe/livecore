use crate::platform::{PeerConnection, PeerConnectionManager, PeerTunnel, PeerConnectionError};

use serde::Deserialize;
use serde_json::Value as JsonValue;

use std::future::Future;
use std::sync::Arc;
use std::pin::Pin;

use anyhow::Result;

use tokio::task::spawn;
use tokio::net::{TcpListener, UnixListener};

use livecore_protocol as proto;
use proto::Uuid;

mod ws;

#[cfg(feature = "ipc_peer")]
mod ipc;

#[cfg(feature = "inmem_peer")]
mod inmem;

pub struct NativePeerConnectionManagerBuilder {
    ws_listener: Option<TcpListener>,
    #[cfg(feature = "ipc_peer")]
    ipc_listener: Option<UnixListener>,
}
impl NativePeerConnectionManagerBuilder {
    pub fn new() -> Self {
        NativePeerConnectionManagerBuilder {
            ws_listener: None,
            #[cfg(feature = "ipc_peer")]
            ipc_listener: None,
        }
    }
    pub fn with_ws_listener(mut self, listener: TcpListener) -> Self {
        self.ws_listener = Some(listener);
        self
    }
    #[cfg(feature = "ipc_peer")]
    pub fn with_ipc_listener(mut self, listener: UnixListener) -> Self {
        self.ipc_listener = Some(listener);
        self
    }
    pub fn build(self) -> NativePeerConnectionManager {
        let ws_matcher = ws::server::WSMatcher::new();

        #[cfg(feature = "ipc_peer")]
        let ipc_matcher = ipc::server::IPCMatcher::new();

        if let Some(listener) = self.ws_listener {
            spawn(ws::server::ws_server(listener, ws_matcher.clone()));
        }

        #[cfg(feature = "ipc_peer")]
        if let Some(listener) = self.ipc_listener {
            spawn(ipc::server::server(listener, ipc_matcher.clone()));
        }

        let manager = NativePeerConnectionManager {
            ws_matcher,
            #[cfg(feature = "ipc_peer")]
            ipc_matcher,
        };

        manager
    }
}

pub struct NativePeerConnectionManager {
    ws_matcher: Arc<ws::server::WSMatcher>,
    #[cfg(feature = "ipc_peer")]
    ipc_matcher: Arc<ipc::server::IPCMatcher>,
}

impl PeerConnectionManager for NativePeerConnectionManager {
    fn start_connect_peer(
        &self,
        peer_tunnel: PeerTunnel,
        conn_type: proto::Connector,
        self_nonce: Uuid,
        peer_nonce: Uuid,
    ) -> Pin<Box<dyn Future<Output = Result<PeerConnection>> + Send>> {
        let ws_matcher = self.ws_matcher.clone();
        #[cfg(feature = "ipc_peer")]
        let ipc_matcher = self.ipc_matcher.clone();

        async fn run(
            ws_matcher: Arc<ws::server::WSMatcher>,
            #[cfg(feature = "ipc_peer")]
            ipc_matcher: Arc<ipc::server::IPCMatcher>,
            peer_tunnel: PeerTunnel,
            conn_type: proto::Connector,
            self_nonce: Uuid,
            peer_nonce: Uuid,
        ) -> Result<PeerConnection> {
            match conn_type {
                proto::Connector::IpcServer => {
                    #[cfg(feature = "ipc_peer")]
                    return Ok(ipc::server::connect(ipc_matcher, self_nonce, peer_nonce).await?);

                    #[cfg(not(feature = "ipc_peer"))]
                    panic!("ipc server not supported");
                },
                proto::Connector::IpcClient(data) => {
                    #[cfg(feature = "ipc_peer")]
                    return Ok(ipc::client::connect(&data.socket_path, self_nonce, peer_nonce).await?);

                    #[cfg(not(feature = "ipc_peer"))]
                    panic!("ipc client not supported");
                },
                proto::Connector::WebsocketServer => {
                    todo!()
                },
                proto::Connector::WebsocketClient(data) => {
                    todo!()
                },
                proto::Connector::WebRTC => {
                    todo!()
                },
            }
        }

        Box::pin(run(
            ws_matcher,
            #[cfg(feature = "ipc_peer")]
            ipc_matcher,
            peer_tunnel,
            conn_type,
            self_nonce,
            peer_nonce
        ))
    }
}

#[cfg(test)]
mod tests {
    use futures::{StreamExt, SinkExt};

    use crate::platform::{PeerTunnel, PeerConnectionManager};
    use super::NativePeerConnectionManager;

    #[tokio::test]
    async fn test_connect() {
        let conn_manager_1 = NativePeerConnectionManager::new();
        let conn_manager_2 = conn_manager_1.clone();

        let a_nonce_1 = vec![1u8; 32];
        let a_nonce_2 = a_nonce_1.clone();
        let b_nonce_1 = vec![2u8; 32];
        let b_nonce_2 = b_nonce_1.clone();

        let h1 = tokio::spawn(async move {
            let dummy_tunnel = PeerTunnel::new_dummy();
            let incoming_fut = conn_manager_1.start_connect_peer(
                dummy_tunnel,
                serde_json::json!({
                    "kind": "websocket_in",
                    "self_nonce": a_nonce_1,
                    "other_nonce": b_nonce_1,
                }),
            );
            let mut peer = incoming_fut.await;

            peer.sink.send(vec![1, 2, 3]).await.map_err(|_| ()).unwrap();
        });

        let h2 = tokio::spawn(async move {
            let dummy_tunnel = PeerTunnel::new_dummy();
            let outgoing_fut = conn_manager_2.start_connect_peer(
                dummy_tunnel,
                serde_json::json!({
                    "kind": "websocket_out",
                    "url": "ws://localhost:6789",
                    "self_nonce": b_nonce_2,
                    "other_nonce": a_nonce_2,
                }),
            );
            let mut peer = outgoing_fut.await;

            let recv = peer.source.next().await.unwrap().map_err(|_| ()).unwrap();
            assert!(recv == vec![1, 2, 3]);
        });

        let (r1, r2) = futures::join!(h1, h2);
        r1.unwrap();
        r2.unwrap();
    }

}
