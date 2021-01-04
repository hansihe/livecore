use tokio::sync::mpsc;

use livecore_protocol as proto;

#[derive(Clone)]
pub struct OrchPacketSender {
    sender: mpsc::UnboundedSender<Vec<u8>>,
}
impl OrchPacketSender {
    pub fn new() -> (Self, mpsc::UnboundedReceiver<Vec<u8>>) {
        let (sender, receiver) = mpsc::unbounded_channel();
        let sender = Self {
            sender,
        };
        (sender, receiver)
    }
    pub fn send<P: Into<proto::OrchClientMsg>>(&mut self, packet: P) {
        let msg: proto::OrchClientMsg = packet.into();
        let serialized = msg.serialize().unwrap();
        self.sender.send(serialized.into()).unwrap();
    }
}
