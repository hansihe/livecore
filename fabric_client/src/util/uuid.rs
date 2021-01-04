use std::convert::TryInto;

use livecore_protocol::Uuid;

pub fn parse_uuid(data: &[u8]) -> Option<Uuid> {
    match data.len() {
        16 => Some(Uuid::from_bytes(data.try_into().unwrap())),
        32 | 36 => {
            std::str::from_utf8(data).ok()
                .and_then(|s| Uuid::parse_str(s).ok())
        },
        _ => None,
    }
}
