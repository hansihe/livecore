use serde::{Deserialize, Serialize};

mod types;
pub use types::uuid::Uuid;
pub use types::hash::Hash;
pub use types::protocol_version::ProtocolVersion;

#[cfg(feature = "jsonschema")]
use schemars::JsonSchema;

pub const VERSION: u32 = 0;

#[macro_export]
macro_rules! impl_from {
    ($enum:ty, $variant:ident, $inner:ty) => {
        impl std::convert::From<$inner> for $enum {
            fn from(inner: $inner) -> Self {
                Self::$variant(inner)
            }
        }
        impl std::convert::TryFrom<$enum> for $inner {
            type Error = ();
            fn try_from(value: $enum) -> Result<Self, ()> {
                type ENUM = $enum;
                match value {
                    ENUM::$variant(inner) => Ok(inner),
                    _ => Err(()),
                }
            }
        }
    };
}

mod orch;
pub use orch::client::*;
pub use orch::server::*;

mod peer;
pub use peer::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "jsonschema", derive(JsonSchema))]
pub struct Challenge {
    /// Challenge for the client to sign with its private key.
    /// A value should be formatted as
    /// `__HANDSHAKE_CHALLENGE__{challenge}{nonce}__HANDSHAKE_CHALLENGE__`
    /// signed, and returned as a `ChallengeResponse`.
    pub challenge: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "jsonschema", derive(JsonSchema))]
pub struct ChallengeResponse {
    /// The response for the challenge from the server.
    pub challenge_response: Vec<u8>,
    pub signature: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "jsonschema", derive(JsonSchema))]
pub struct IpcClient {
    pub socket_path: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "jsonschema", derive(JsonSchema))]
pub struct WebsocketClient {
    url: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "jsonschema", derive(JsonSchema))]
#[serde(tag = "ty", rename_all = "snake_case")]
pub enum PeerConnectionType {
    IpcClient,
    IpcServer,
    WebsocketClient,
    WebsocketServer,
    WebRTC,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "jsonschema", derive(JsonSchema))]
#[serde(tag = "ty", rename_all = "snake_case")]
pub enum Connector {
    IpcClient(IpcClient),
    IpcServer,
    WebsocketClient(WebsocketClient),
    WebsocketServer,
    WebRTC,
}


//pub fn serialize(message: &Message) -> bincode::Result<Vec<u8>> {
//    bincode::serialize(message)
//}
//
//pub fn serialize_into<W: Write>(writer: W, message: &Message) -> bincode::Result<()> {
//    bincode::serialize_into(writer, message)
//}
//
//pub fn deserialize(bytes: &[u8]) -> bincode::Result<Message> {
//    bincode::deserialize(bytes)
//}
