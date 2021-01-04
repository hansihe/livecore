use std::time::Duration;

use tokio::time::timeout_at;

use futures::sink::SinkExt;
use futures::stream::StreamExt;

use tokio_tungstenite::tungstenite::Message as TMessage;
use tokio_tungstenite::tungstenite;

use crate::platform::{PeerConnection, PeerConnectionManager, PeerTunnel, PeerConnectionError};

pub async fn do_start_connect_outgoing(url: String, self_nonce: Vec<u8>, other_nonce: Vec<u8>) -> PeerConnection {
    let timeout_time = tokio::time::Instant::now() + Duration::from_millis(10000);

    let res = timeout_at(timeout_time, tokio_tungstenite::connect_async(url)).await;
    let (mut ws_stream, _response) = match res {
        Ok(Ok((ws_stream, response))) => (ws_stream, response),
        Ok(Err(_)) => todo!(),
        Err(_) => {
            // TODO Timeout
            todo!()
        },
    };

    // TODO error
    ws_stream.send(TMessage::Binary(self_nonce)).await.unwrap();

    // Expect a single message with a 32 byte nonce from the peer.
    let nonce = loop {
        match tokio::time::timeout_at(timeout_time, ws_stream.next()).await {
            Ok(Some(Ok(TMessage::Binary(nonce)))) if nonce.len() == 32 => {
                log::info!("received nonce for outgoing WS connection");
                break nonce;
            }

            // No item in stream, socket closed immediately.
            Ok(None) => {
                log::warn!("outgoing WS closed without nonce");
                todo!()
            }
            Ok(Some(Err(ws_error))) => {
                log::warn!("failed to receive handhake from outgoing WS peer connection ({})", ws_error);
                todo!()
            }
            Ok(Some(Ok(TMessage::Close(_)))) => unreachable!(),
            Ok(Some(Ok(TMessage::Binary(_)))) | Ok(Some(Ok(TMessage::Text(_)))) => {
                log::warn!("received invalid handshake for outgoing peer WS");
                todo!()
            }
            Ok(_) => continue,
            Err(_) => {
                log::warn!("WS peer handshake timed out");
                todo!()
            }
        }
    };

    if nonce != other_nonce {
        // nonce check failed, connection failed
        todo!()
    }

    let (sink, source) = ws_stream.split();

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
}
