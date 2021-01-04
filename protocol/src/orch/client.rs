use serde::{Deserialize, Serialize};

use crate::{ProtocolVersion, PeerConnectionType, Challenge, ChallengeResponse, Uuid, impl_from};

/// When establishing a fabric connection, this message must be sent initially
/// by the client.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "jsonschema", derive(JsonSchema))]
pub struct ClientHandshake {
    /// Protocol version in use.
    /// Right now the fabric can only operate if the client and server
    /// versions match, so this functions as a validation.
    pub version: ProtocolVersion,

    /// The set of classes for a node.
    /// Which values are supported here depends on the orchestrator.
    pub node_classes: Vec<String>,

    /// The types of peer connections the peer is capable of establishing.
    /// All nodes should be capable of, at a bare minimum, `WebsocketClient`.
    pub peer_connection_capabilities: Vec<PeerConnectionType>,

    /// A token used for potential authorization or authentication of the
    /// client.
    pub token: Option<String>,

    /// When a fabric client starts up, it should generate a
    /// `ECDSA_P256_SHA256_FIXED` keypair, and send its public key.
    pub pubkey: Vec<u8>,

    pub challenge: Challenge,
}

/// Sent by the client after it has received a `ServerHandshake`.
/// The handshake procedure is complete after this message is received.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "jsonschema", derive(JsonSchema))]
pub struct ClientHandshakeFinish {
    pub challenge_response: ChallengeResponse,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "jsonschema", derive(JsonSchema))]
pub struct PeerConnectionFailed {
    /// The peer we attempted to connect to.
    pub peer_uuid: Uuid,
    /// A human readable reason for the connection failure.
    /// Mainly used for debugging.
    pub fail_reason: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "jsonschema", derive(JsonSchema))]
pub struct PeerConnectionSuccess {
    pub peer_uuid: Uuid,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "jsonschema", derive(JsonSchema))]
pub struct PeerConnectionDisconnected {
    pub peer_uuid: Uuid,
    /// A human readable reason for the connection failure.
    /// Mainly used for debugging.
    pub fail_reason: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "jsonschema", derive(JsonSchema))]
#[serde(tag = "ty", rename_all = "snake_case")]
pub enum OrchClientMsg {
    ClientHandshake(ClientHandshake),
    ClientHandshakeFinish(ClientHandshakeFinish),

    PeerConnectionFailed(PeerConnectionFailed),
    PeerConnectionSuccess(PeerConnectionSuccess),
    PeerConnectionDisconnected(PeerConnectionDisconnected),
}
impl OrchClientMsg {
    pub fn serialize(&self) -> serde_json::Result<Vec<u8>> {
        serde_json::to_vec(self)
    }
    pub fn deserialize(string: &[u8]) -> serde_json::Result<Self> {
        serde_json::from_slice(string)
    }
}
impl_from!(OrchClientMsg, ClientHandshake, ClientHandshake);
impl_from!(OrchClientMsg, ClientHandshakeFinish, ClientHandshakeFinish);
impl_from!(OrchClientMsg, PeerConnectionFailed, PeerConnectionFailed);
impl_from!(OrchClientMsg, PeerConnectionSuccess, PeerConnectionSuccess);
impl_from!(OrchClientMsg, PeerConnectionDisconnected, PeerConnectionDisconnected);
