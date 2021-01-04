//! Implementation of incoming WS peer connector.
//!
//! Uses a `Matcher` to match incoming WS connections to connection requests
//! from the orchestrator. A timeout is used to prevent DOS attacks.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use tokio::net::{TcpListener, TcpStream};
use tokio::task::spawn;
use tokio::time::timeout_at;

use futures::sink::SinkExt;
use futures::stream::StreamExt;

use tokio_tungstenite::{tungstenite::Message as TMessage, WebSocketStream};
use tokio_tungstenite::tungstenite;

use crate::util::matcher::{HoldSuccess, HoldError, PermitError, Matcher};
use crate::platform::{PeerConnection, PeerConnectionManager, PeerTunnel, PeerConnectionError};

pub type WSMatcher = Matcher<Vec<u8>, WebSocketStream<TcpStream>>;

async fn accept_ws(
    matcher: Arc<WSMatcher>,
    stream: TcpStream,
    addr: SocketAddr
) {
    let timeout_time = tokio::time::Instant::now() + Duration::from_millis(10000);

    let mut ws_stream =
        match timeout_at(timeout_time, tokio_tungstenite::accept_async(stream)).await {
            Ok(Ok(ws_stream)) => ws_stream,
            Ok(Err(_)) => {
                log::warn!("WS peer handshake fail");
                return;
            }
            Err(_) => {
                log::warn!("WS peer handshake timed out");
                return;
            }
        };

    // Expect a single message with a 32 byte nonce from the peer.
    let (nonce, ws_stream) = loop {
        match tokio::time::timeout_at(timeout_time, ws_stream.next()).await {
            Ok(Some(Ok(TMessage::Binary(nonce)))) if nonce.len() == 32 => {
                log::info!(
                    "received handshake noce for incoming peer WS, posting to matcher ({})",
                    addr
                );
                break (nonce, ws_stream);
            }

            // No item in stream, socket closed immediately.
            Ok(None) => {
                log::warn!(
                    "incoming WS peer connection closed without handshake message ({})",
                    addr
                );
                return;
            }
            Ok(Some(Err(ws_error))) => {
                log::warn!(
                    "failed to receive handshake on WS peer connection ({}) ({})",
                    addr,
                    ws_error
                );
                return;
            }
            Ok(Some(Ok(TMessage::Close(_)))) => unreachable!(),
            Ok(Some(Ok(TMessage::Binary(_)))) | Ok(Some(Ok(TMessage::Text(_)))) => {
                log::warn!("received invalid handshake for incoming peer WS");
                return;
            }
            Ok(_) => continue,
            Err(_) => {
                log::warn!("WS peer handshake timed out");
                return;
            }
        }
    };

    match matcher.send(&nonce, timeout_time, ws_stream).await {
        Ok(success) => {
            log::info!("held connection accepted ({:?})", success);
        },
        Err(error) => {
            log::warn!("held connection dropped ({:?})", error);
        },
    }
}

pub async fn ws_server(listener: TcpListener, matcher: Arc<WSMatcher>) {
    while let Ok((stream, addr)) = listener.accept().await {
        let matcher = matcher.clone();

        spawn(accept_ws(matcher, stream, addr));
    }
}

pub async fn do_start_connect_incoming(matcher: Arc<WSMatcher>, self_nonce: Vec<u8>, other_nonce: Vec<u8>) -> PeerConnection {
    let timeout_time = tokio::time::Instant::now() + Duration::from_millis(10000);

    match matcher.receive(&other_nonce, timeout_time).await {
        Ok(conn) => {
            let (mut sink, source) = conn.split();

            sink.send(TMessage::Binary(self_nonce)).await.unwrap();

            let sink = sink
                .with(|bin| async { Ok(TMessage::Binary(bin)) })
                .sink_map_err(|_err: tungstenite::Error| PeerConnectionError::Wat);

            let source = source
                .map(|val| {
                    match val {
                        Ok(TMessage::Binary(bin)) => Ok(bin),
                        Ok(_) => todo!(),
                        Err(_) => Err(PeerConnectionError::Wat),
                    }
                });

            PeerConnection {
                sink: Box::pin(sink),
                source: Box::pin(source),
            }
        },
        Err(error) => todo!(),
    }
}
