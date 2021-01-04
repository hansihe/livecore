use std::sync::Mutex;
use std::collections::HashMap;
use std::pin::Pin;
use std::task::{Poll, Context};

use tokio::sync::{mpsc, Notify};

use futures::{Stream, Sink};

use livecore_protocol as proto;
use proto::Uuid;

use crate::platform::{PeerConnection, PeerConnectionError};

lazy_static::lazy_static! {
    static ref MATCHER: Mutex<HashMap<(Uuid, Uuid), PeerConnection>> = {
        Mutex::new(HashMap::new())
    };
}

struct MpscSink(mpsc::UnboundedSender<Vec<u8>>);
impl Sink<Vec<u8>> for MpscSink {
    type Error = PeerConnectionError;
    fn poll_ready(
        self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Result<(), Self::Error>> {
        todo!()
    }
    fn start_send(
        self: Pin<&mut Self>,
        item: Vec<u8>,
    ) -> Result<(), Self::Error> {
        todo!()
    }
    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Result<(), Self::Error>> {
        todo!()
    }
    fn poll_close(
        self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Result<(), Self::Error>> {
        todo!()
    }
}

struct MpscSource(mpsc::UnboundedReceiver<Vec<u8>>);
impl Stream for MpscSource {
    type Item = Result<Vec<u8>, PeerConnectionError>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Result<Vec<u8>, PeerConnectionError>>> {
        match self.0.poll_recv(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Some(inner)) => Poll::Ready(Some(Ok(inner))),
            Poll::Ready(None) => Poll::Ready(None),
        }
    }
}

pub (super) fn do_connect(self_nonce: Uuid, other_nonce: Uuid) -> PeerConnection {
    let mut lock = (*MATCHER).lock().unwrap();

    if let Some(conn) = lock.remove(&(other_nonce, self_nonce)) {
        conn
    } else {
        let (s1, r1) = mpsc::unbounded_channel();
        let (s2, r2) = mpsc::unbounded_channel();

        lock.insert(
            (self_nonce, other_nonce),
            PeerConnection {
                sink: Box::pin(MpscSink(s1)),
                source: Box::pin(MpscSource(r2)),
            },
        );

        PeerConnection {
            sink: Box::pin(MpscSink(s2)),
            source: Box::pin(MpscSource(r1)),
        }
    }
}
