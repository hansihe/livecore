use super::super::{PeerConnection, PeerConnectionManager, PeerTunnel, PeerConnectionError};

use serde::Deserialize;
use serde_json::Value as JsonValue;

use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use std::pin::Pin;

use tokio::net::{TcpListener, TcpStream};
use tokio::task::spawn;
use tokio::time::timeout_at;

use futures::sink::SinkExt;
use futures::stream::StreamExt;

use tokio_tungstenite::{tungstenite::Message as TMessage, WebSocketStream};
use tokio_tungstenite::tungstenite;

use crate::matcher::{HoldSuccess, HoldError, PermitError, Matcher};

mod ws_in;
mod ws_out;

#[derive(Clone)]
pub struct NativePeerConnectionManager {
    matcher: Arc<ws_in::WSMatcher>,
}

impl NativePeerConnectionManager {
    pub fn new() -> Self {
        let matcher = ws_in::WSMatcher::new();

        spawn(ws_in::ws_server(matcher.clone()));

        let manager = NativePeerConnectionManager {
            matcher,
        };

        manager
    }
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Config {
    WebsocketIn {
        self_nonce: Vec<u8>,
        other_nonce: Vec<u8>,
    },
    WebsocketOut {
        url: String,
        self_nonce: Vec<u8>,
        other_nonce: Vec<u8>,
    }
}

impl PeerConnectionManager for NativePeerConnectionManager {
    fn start_connect_peer(
        &self,
        peer_tunnel: PeerTunnel,
        config: JsonValue,
    ) -> Pin<Box<dyn Future<Output = PeerConnection> + Send>> {
        let matcher = self.matcher.clone();

        async fn run(matcher: Arc<ws_in::WSMatcher>, peer_tunnel: PeerTunnel, config: JsonValue) -> PeerConnection {
            let config: Config = serde_json::from_value(config).unwrap();

            match config {
                Config::WebsocketIn { self_nonce, other_nonce } =>
                    ws_in::do_start_connect_incoming(matcher, self_nonce, other_nonce).await,
                Config::WebsocketOut { self_nonce, other_nonce, url } =>
                    ws_out::do_start_connect_outgoing(url, self_nonce, other_nonce).await,
            }
        }

        Box::pin(run(matcher, peer_tunnel, config))
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
