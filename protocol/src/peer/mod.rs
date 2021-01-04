use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "jsonschema", derive(JsonSchema))]
pub struct FragmentData {
    data: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "jsonschema", derive(JsonSchema))]
pub enum PeerMsg {
    StreamData { data: Vec<u8> },

    FragmentData(FragmentData),
}
impl PeerMsg {
    pub fn serialize(&self) -> bincode::Result<Vec<u8>> {
        bincode::serialize(self)
    }
    pub fn deserialize(string: &[u8]) -> bincode::Result<Self> {
        bincode::deserialize(string)
    }
}
