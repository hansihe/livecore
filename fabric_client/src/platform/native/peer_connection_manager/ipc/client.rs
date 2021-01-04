use std::path::Path;
use std::time::{Instant, Duration};
use std::io::Write;

use tokio::net::UnixStream;
use tokio::net::unix::SocketAddr;
use tokio::time::timeout_at;
use tokio_util::codec::{Framed, LengthDelimitedCodec};

use futures::{StreamExt, SinkExt};

use anyhow::{Result, Context, ensure, bail};

use livecore_protocol::Uuid;

use crate::platform::{PeerConnection, PeerConnectionManager, PeerTunnel, PeerConnectionError};

pub async fn connect<P: AsRef<Path>>(path: P, self_nonce: Uuid, other_nonce: Uuid) -> Result<PeerConnection> {
    let timeout_time = Instant::now() + Duration::from_millis(10000);

    let stream = UnixStream::connect(path).await.context("failed to open unix socket")?;
    let mut framed = Framed::new(stream, LengthDelimitedCodec::new());

    log::info!("connected to peer IPC socket");

    let (mut sink, mut source) = framed.split();

    let mut buf = Vec::new();
    write!(buf, "{}", self_nonce).unwrap();
    sink.send(buf.into()).await.context("failed to send nonce to peer")?;

    let uuid = match timeout_at(timeout_time.into(), source.next()).await {
        Ok(Some(Ok(inner))) if inner.len() == 32 || inner.len() == 36 => {
            if let Some(uuid) = crate::util::uuid::parse_uuid(&inner) {
                uuid
            } else {
                log::warn!("IPC peer sent invalid handshake nonce");
                bail!("IPC peer send invalid handshake nonce");
            }
        }
        Ok(Some(Ok(inner))) => {
            bail!("mailformed nonce");
        }
        Ok(None) => {
            log::warn!("incoming IPC peer connection closed without handshake UUID");
            bail!("IPC peer connection closed without handshake nonce");
        }
        Err(_) => {
            log::warn!("IPC peer handshake timed out");
            bail!("IPC peer connection handshake timed out");
        }
        Ok(Some(Err(error))) => {
            log::warn!("IPC peer connection error");
            return Err(error).context("IPC peer connection error");
        }
    };

    ensure!(uuid == other_nonce, "received non-matching nonce on peer handshake");

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

    log::info!("handshaked to peer IPC socket");

    Ok(PeerConnection {
        sink: Box::pin(sink),
        source: Box::pin(source),
    })
}
