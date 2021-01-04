use std::collections::HashSet;

use livecore_protocol as proto;
use proto::Uuid;

use ring::signature::UnparsedPublicKey;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum KeyCapability {
    /// The key has permission to originate a stream with the given UUID.
    /// If the node is informed that a node has the capability to originate a
    /// stream, this node must accept stream manifests, object manifests and
    /// fragment manifests signed with this nodes pubkey that are received over
    /// peer connections.
    OriginateStream(Uuid),
}

pub struct Key {
    key: UnparsedPublicKey<Vec<u8>>,
    capabilities: HashSet<KeyCapability>,
}
