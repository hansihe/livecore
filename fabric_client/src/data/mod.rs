//! Data layer.
//!
//! Responsible for managing fragments and objects stored on a client.
//!
//! An object consists of zero of more fragments. A fragment is an atomic unit
//! of data distribution. When a client has all the fragments of an object,
//! it may assemble the full object.
//!
//! Both objects and fragments may be marked as live. A live entity is kept in
//! memory, while a dead one may be garbage collected.
//!
//! If an object is marked as live, all its fragments are kept alive by it.

use std::collections::{HashMap, HashSet};
use std::time::Instant;
use std::sync::atomic::AtomicUsize;

pub mod fragment_buffer;
use fragment_buffer::{FragSize, FragmentBuffer, RootBuffer};

use livecore_protocol as proto;
use proto::Hash;

#[derive(Debug, Eq, PartialEq)]
enum FragmentState {
    Expecting,
    Present,
}

#[derive(Debug)]
struct Fragment {
    hash: Hash,
    state: FragmentState,

    tmp_data: Option<Vec<u8>>,
    buffers: Vec<FragmentBuffer>,
}

struct Object {
    hash: Hash,
    buffer: RootBuffer,

    fragment_hashes: Vec<Hash>,
    fragment_hash_to_idx: HashMap<Hash, usize>,
}

pub struct DataManager {
    objects: HashMap<Hash, Object>,
    fragments: HashMap<Hash, Fragment>,
}

impl DataManager {

    pub fn new() -> Self {
        Self {
            objects: HashMap::new(),
            fragments: HashMap::new(),
        }
    }

    pub fn handle_object_manifest(&mut self, manifest: proto::ObjectManifest) {
        let root = RootBuffer::new(manifest.size, FragSize(manifest.fragment_size));

        for (idx, fragment) in manifest.fragments.iter().enumerate() {
            let mut frag_buf = root.claim(idx).unwrap();

            let frag = self.fragments.entry(fragment.hash).or_insert(Fragment {
                hash: fragment.hash,
                state: FragmentState::Expecting,
                tmp_data: None,
                buffers: vec![],
            });

            if let Some(data) = frag.tmp_data.take() {
                frag_buf.fill(&data);
            } else if frag.buffers.len() > 0 {
                let prev_buf = &frag.buffers[0];
                if frag.state == FragmentState::Present {
                    frag_buf.fill(prev_buf.as_ref());
                }
            }
            frag.buffers.push(frag_buf);
        }

        let object = Object {
            hash: manifest.hash,
            buffer: root.clone(),

            fragment_hashes: manifest.fragments.iter().map(|f| f.hash).collect(),
            fragment_hash_to_idx: manifest.fragments.iter().enumerate().map(|(i, f)| (f.hash, i)).collect(),
        };

        assert!(!self.objects.contains_key(&manifest.hash));
        self.objects.insert(manifest.hash, object);
    }

}
