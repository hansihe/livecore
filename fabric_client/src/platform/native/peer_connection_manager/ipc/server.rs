use std::sync::Arc;
use std::path::Path;
use std::time::{Duration, Instant};
use std::io::Write;

use anyhow::{Result, Context};

use tokio::spawn;
use tokio::net::{UnixStream, UnixListener};
use tokio::net::unix::SocketAddr;
use tokio::time::timeout_at;
use tokio_util::codec::{Framed, LengthDelimitedCodec};

use futures::{StreamExt, SinkExt};

use livecore_protocol as proto;
use proto::Uuid;

use crate::util::matcher::{HoldSuccess, HoldError, PermitError, Matcher};
use crate::platform::{PeerConnection, PeerConnectionManager, PeerTunnel, PeerConnectionError};

pub type IPCMatcher = Matcher<Uuid, Framed<UnixStream, LengthDelimitedCodec>>;

async fn accept_ipc(
    matcher: Arc<IPCMatcher>,
    stream: UnixStream,
    addr: SocketAddr
) {
    let timeout_time = Instant::now() + Duration::from_millis(10000);

    let mut framed = Framed::new(stream, LengthDelimitedCodec::new());

    let uuid = match timeout_at(timeout_time.into(), framed.next()).await {
        Ok(Some(Ok(inner))) if inner.len() == 32 || inner.len() == 36 => {
            if let Some(uuid) = crate::util::uuid::parse_uuid(&inner) {
                uuid
            } else {
                log::warn!("IPC peer sent invalid handshake UUID");
                return;
            }
        }
        Ok(None) => {
            log::warn!("incoming IPC peer connection closed without handshake UUID");
            return;
        }
        Err(_) => {
            log::warn!("IPC peer handshake timed out");
            return;
        }
        _ => {
            log::warn!("IPC peer connection error");
            return;
        }
    };

    match matcher.send(&uuid, timeout_time.into(), framed).await {
        Ok(success) => {
            log::info!("held connection accepted ({:?})", success);
        },
        Err(error) => {
            log::warn!("held connection dropped ({:?})", error);
        }
    }
}

pub async fn server(listener: UnixListener, matcher: Arc<IPCMatcher>) {
    while let Ok((stream, addr)) = listener.accept().await {
        let matcher = matcher.clone();
        spawn(accept_ipc(matcher, stream, addr));
    }
}

pub async fn connect(matcher: Arc<IPCMatcher>, self_nonce: Uuid, other_nonce: Uuid) -> Result<PeerConnection> {
    let timeout_time = Instant::now() + Duration::from_millis(10000);

    log::info!("waiting for incoming peer IPC connection");
    let conn = matcher.receive(&other_nonce, timeout_time.into())
           .await
           .context("failed to get connection from matcher")?;

    let (mut sink, source) = conn.split();

    let mut buf = Vec::new();
    write!(buf, "{}", self_nonce).unwrap();
    sink.send(buf.into()).await.context("failed to send nonce to peer")?;

    let sink = sink
        .with(|bin: Vec<u8>| async { Ok(bin.into()) })
        .sink_map_err(|_err: std::io::Error| PeerConnectionError::Wat);

    let source = source
        .map(|val| {
            match val {
                Ok(bin) => Ok(bin.as_ref().to_owned()),
                Err(_) => Err(PeerConnectionError::Wat),
            }
        });

    Ok(PeerConnection {
        sink: Box::pin(sink),
        source: Box::pin(source),
    })
}
