#[cfg(not(feature = "jsonschema"))]
pub use uuid::Uuid;

#[cfg(feature = "jsonschema")]
pub use self::shim::Uuid;
#[cfg(feature = "jsonschema")]
mod shim {
    use uuid::Uuid as OUuid;
    use schemars::{JsonSchema, gen::SchemaGenerator, schema::{Schema, SchemaObject, InstanceType}};

    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[repr(transparent)]
    pub struct Uuid(OUuid);

    impl JsonSchema for Uuid {
        fn schema_name() -> String {
            "uuid".to_owned()
        }
        fn json_schema(_: &mut SchemaGenerator) -> Schema {
            let mut schema = SchemaObject::default();
            schema.instance_type = Some(InstanceType::String.into());
            schema.format = Some("uuid".into());
            schema.into()
        }
    }
}
