use livecore_protocol as proto;
use proto::Uuid;

mod key;

pub(crate) struct PeerState {
    uuid: Uuid,
    key: key::Key,
}
