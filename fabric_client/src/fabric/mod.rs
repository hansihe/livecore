use tokio::sync::mpsc;

use livecore_protocol as proto;

mod builder;
mod packet_sender;
mod state;

pub(crate) use state::{FabricState, FabricProtoState};

pub use builder::FabricBuilder;
pub use packet_sender::OrchPacketSender;

pub struct Fabric {
    fabric_packet_in: mpsc::Sender<proto::OrchServerMsg>,
}
impl Fabric {
    pub async fn handle_fabric_packet(&self, msg: proto::OrchServerMsg) {
        self.fabric_packet_in.send(msg).await.unwrap()
    }
}
