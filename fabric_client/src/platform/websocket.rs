use std::future::Future;
use std::pin::Pin;

use futures::{Stream, Sink};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WebsocketMessage {
    Text(String),
    Binary(Vec<u8>),
}
impl WebsocketMessage {
    pub fn text(&self) -> Option<&str> {
        match self {
            WebsocketMessage::Text(text) => Some(text),
            _ => None,
        }
    }
    pub fn bytes(&self) -> &[u8] {
        match self {
            WebsocketMessage::Text(text) => text.as_bytes(),
            WebsocketMessage::Binary(bin) => bin,
        }
    }
}
impl From<Vec<u8>> for WebsocketMessage {
    fn from(bin: Vec<u8>) -> WebsocketMessage {
        WebsocketMessage::Binary(bin)
    }
}

pub trait WebsocketMeta {
    fn close(&self) -> Pin<Box<dyn Future<Output = ()>>>;
}

pub struct WebsocketStream {
    pub meta: Box<dyn WebsocketMeta + Send>,
    pub sink: Pin<Box<dyn Sink<WebsocketMessage, Error = ()> + Send>>,
    pub source: Pin<Box<dyn Stream<Item = Result<WebsocketMessage, ()>> + Send>>,
}
