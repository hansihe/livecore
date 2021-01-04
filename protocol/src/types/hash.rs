use std::fmt;
use std::fmt::Write;

use serde::{Deserialize, Serialize};
use serde::{Serializer, Deserializer};

#[cfg(feature = "jsonschema")]
use schemars::JsonSchema;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Hash(
    pub [u8; 32]
);

impl Hash {

    pub fn to_string(&self) -> String {
        let mut buf = String::new();
        for byte in self.0.iter() {
            write!(buf, "{:02x?}", byte).unwrap();
        }
        buf
    }

    pub fn parse_str(string: &str) -> Option<Self> {
        fn hex_to_num(num: u8) -> Option<u8> {
            if num >= '0' as u8 && num <= '9' as u8 {
                Some(num - '0' as u8)
            } else if num >= 'a' as u8 && num <= 'f' as u8 {
                Some(num - 'a' as u8 + 10)
            } else {
                None
            }
        }

        fn hex_to_u8(hex: &[u8]) -> Option<u8> {
            debug_assert!(hex.len() == 2);
            let nibble1 = hex_to_num(hex[0]);
            let nibble2 = hex_to_num(hex[1]);

            if let (Some(n1), Some(n2)) = (nibble1, nibble2) {
                Some(n1 << 4 | n2)
            } else {
                None
            }
        }

        let mut bytes = [0u8; 32];
        let mut idx = 0;
        for chunk in string.as_bytes().chunks(2) {
            if chunk.len() != 2 {
                return None;
            }
            if let Some(byte) = hex_to_u8(chunk) {
                if let Some(elem) = bytes.get_mut(idx) {
                    *elem = byte;
                    idx += 1;
                    continue;
                }
            }
            return None;
        }

        if idx != 32 {
            return None;
        }

        Some(Hash(bytes))
    }

}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Hash(")?;
        for num in self.0.iter() {
            write!(f, "{:x?}", num)?
        }
        write!(f, ")")
    }
}

impl Serialize for Hash {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            let string = self.to_string();
            serializer.serialize_str(&string)
        } else {
            serializer.serialize_bytes(&self.0)
        }
    }
}

impl<'de> Deserialize<'de> for Hash {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error;

        if deserializer.is_human_readable() {
            let string = String::deserialize(deserializer)?;
            Hash::parse_str(&string).ok_or_else(|| {
                D::Error::custom("invalid hash")
            })
        } else {
            let data = Vec::deserialize(deserializer)?;
            let mut buf = [0u8; 32];
            if data.len() == 32 {
                (&mut buf).copy_from_slice(&data);
                Ok(Hash(buf))
            } else {
                Err(D::Error::custom("invalid hash"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{to_value, from_value};
    use super::Hash;

    #[test]
    fn parsing() {
        assert!(Hash::parse_str("0000000000000000000000000000000000000000000000000000000000000000").is_some());
        assert!(Hash::parse_str("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff").is_some());
        assert!(Hash::parse_str("fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff").is_none());
        assert!(Hash::parse_str("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff").is_none());
        assert!(Hash::parse_str("fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff").is_none());
        assert!(Hash::parse_str("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff").is_none());
    }

    #[test]
    fn round_trip() {
        let hash = Hash([
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
            16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 222, 255,
        ]);
        let value = to_value(hash).unwrap();
        let hash_back: Hash = from_value(value).unwrap();

        assert_eq!(hash, hash_back);
    }
}
