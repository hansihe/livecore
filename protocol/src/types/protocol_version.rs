use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error};

#[derive(Default, Copy, Clone)]
pub struct ProtocolVersion;

impl std::fmt::Debug for ProtocolVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", crate::VERSION)
    }
}

impl Serialize for ProtocolVersion {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u32(crate::VERSION)
    }
}
impl<'de> Deserialize<'de> for ProtocolVersion {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let ver: u32 = Deserialize::deserialize(deserializer)?;

        if ver == crate::VERSION {
            Ok(ProtocolVersion)
        } else {
            Err(D::Error::custom(format!("invalid protocol version (expected {}, got {})", crate::VERSION, ver)))
        }
    }
}

#[cfg(feature = "jsonschema")]
use schemars::{JsonSchema, gen::SchemaGenerator, schema::{Schema, SchemaObject}};

#[cfg(feature = "jsonschema")]
impl JsonSchema for ProtocolVersion {
    fn schema_name() -> String {
        "protocol_version".to_owned()
    }
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        let mut schema = SchemaObject::default();

        let ver = crate::VERSION;
        schema.const_value = Some(serde_json::json!(ver));

        schema.into()
    }
}
