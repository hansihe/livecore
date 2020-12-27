use std::borrow::Cow;
use std::io::Write;

use serde::{Deserialize, Serialize};

// TODO: Fix copying on deserializing Cow
// https://play.rust-lang.org/?version=nightly&mode=debug&edition=2018&gist=418dd6b98dfa62d43c4cc7fa8b7ea0d6

#[derive(Serialize, Deserialize, Debug)]
pub enum Message {
    StreamData { data: Vec<u8> },
}

pub fn serialize(message: &Message) -> bincode::Result<Vec<u8>> {
    bincode::serialize(message)
}

pub fn serialize_into<W: Write>(writer: W, message: &Message) -> bincode::Result<()> {
    bincode::serialize_into(writer, message)
}

pub fn deserialize(bytes: &[u8]) -> bincode::Result<Message> {
    bincode::deserialize(bytes)
}
