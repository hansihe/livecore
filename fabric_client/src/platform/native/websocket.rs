use std::future::Future;
use std::pin::Pin;

use tokio::sync::mpsc;
use futures::{StreamExt, SinkExt};

use tokio_tungstenite::tungstenite::{self, Message as TMessage};

use crate::platform::{WebsocketMessage, WebsocketStream, WebsocketMeta};

#[derive(Clone)]
struct NativeMeta {
    //channel: mpsc::Sender<TMessage>,
}
impl WebsocketMeta for NativeMeta {
    fn close(&self) -> Pin<Box<dyn Future<Output = ()>>> {
        todo!()
        //let chan = self.channel.clone();
        //Box::pin(async {
        //    chan.send(TMessage::Close(None)).await.unwrap();
        //})
    }
}

pub async fn connect(url: String) -> Result<WebsocketStream, ()> {
    let res = tokio_tungstenite::connect_async(url).await;

    match res {
        Ok((ws_stream, _response)) => {
            let (sink, source) = ws_stream.split();

            //let (meta_send, meta_recv) = mpsc::channel(1);

            let sink = sink
                .with(|msg| async {
                    match msg {
                        WebsocketMessage::Text(text) => Ok(TMessage::Text(text)),
                        WebsocketMessage::Binary(bin) => Ok(TMessage::Binary(bin)),
                    }
                })
                .sink_map_err(|_err: tungstenite::Error| ());

            let source = source.map(|res| {
                match res {
                    Ok(TMessage::Text(text)) => Ok(WebsocketMessage::Text(text)),
                    Ok(TMessage::Binary(text)) => Ok(WebsocketMessage::Binary(text)),
                    Ok(_) => todo!(),
                    Err(_) => Err(()),
                }
            });

            Ok(WebsocketStream {
                meta: Box::new(NativeMeta {}),
                sink: Box::pin(sink),
                source: Box::pin(source),
            })
        }
        Err(err) => {
            log::error!("{:?}", err);
            todo!()
        }
    }
}
