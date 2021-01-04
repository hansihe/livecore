use std::sync::{Arc, atomic::{AtomicU8, Ordering}};

/// Specified fragment size for an object.
/// Specified in terms of `2^n`.
#[derive(Debug, Copy, Clone)]
pub struct FragSize(pub u32);
impl FragSize {
    /// Returns the real size of each fragment.
    pub fn size(&self) -> usize {
        debug_assert!(self.0 != 0);
        2usize.pow(self.0)
    }

    /// Calculates of fragments needed to represent something of the given size.
    pub fn needed_fragments(&self, size: usize) -> usize {
        let frag_size = self.size();
        (size + frag_size - 1) / frag_size
    }
}

// Each fragment in a root buffer has a state represented as an atomic.
//
// State is represented along two dimensions:
//                    Unclaimed    Claimed
//                  +-----------+-----------+
//   Uninitialized: |     0     |     2     |
//                  +-----------+-----------+
//         Mutable: |     1     |     3     |
//                  +-----------+-----------+
//          Sealed: |     4     |     4     |
//                  +-----------+-----------+
//
// ## Mutability axis
// Uninitialized:
//   The fragment is currently uninitialized memory, and may not be read.
// Mutable:
//   The fragment is mutable, but can only be owned by a single `FragmentBuffer`
//   at a time.
// Sealed:
//   The fragment is read only, and may never be changed again. Once a fragment
//   enters this state, it will never leave it. Any number of `FragmentBuffer`s
//   or other readers may reference the fragment memory.
//
// ## Claim axis
// Unclaimed:
//   No `FragmentBuffer` currently exists with ownership, and the fragment may
//   be claimed.
// Claimed:
//   There exists a `FragmentBuffer` with ownership of the fragment memory.
//
// ## Transitions
// Fragments always start as unclaimed, and may be either uninitialized or
// pre-set to data.
//
// Once a fragment goes from UNINITIALIZED to MUTABLE, it will usually not
// transition back, but it may.
//
// Once a fragment transitions from MUTABLE to SEALED, it may never change state
// in the future.
//
// A fragment may transition back and fourth between UNCLAIMED and CLAIMED,
// as `FragmentBuffer`s are created and dropped.
//
const FRAG_UNCLAIMED_UNINITIALIZED: u8 = 0;
const FRAG_UNCLAIMED_MUTABLE: u8 = 1;
const FRAG_CLAIMED_UNINITIALIZED: u8 = 2;
const FRAG_CLAIMED_MUTABLE: u8 = 3;
const FRAG_SEALED: u8 = 4;

#[derive(Clone)]
pub struct RootBuffer(Arc<RootBufferInner>);

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum FragmentState {
    Uninitialized,
    Mutable,
    Sealed,
}

impl RootBuffer {
    pub fn new(size: usize, fragment_size: FragSize) -> Self {
        let num_fragments = fragment_size.needed_fragments(size);

        let layout = std::alloc::Layout::from_size_align(
            size, fragment_size.size()).unwrap();
        let backing = unsafe { std::alloc::alloc(layout) };

        let inner = RootBufferInner {
            fragment_size,
            size,
            backing,
            claims: {
                let mut v = Vec::with_capacity(num_fragments);
                for _ in 0..num_fragments {
                    v.push(AtomicU8::new(FRAG_UNCLAIMED_UNINITIALIZED));
                }
                v
            },
        };

        RootBuffer(Arc::new(inner))
    }

    pub fn new_zeroed(size: usize, fragment_size: FragSize) -> Self {
        let num_fragments = fragment_size.needed_fragments(size);

        let layout = std::alloc::Layout::from_size_align(
            size, fragment_size.size()).unwrap();
        let backing = unsafe { std::alloc::alloc_zeroed(layout) };

        let inner = RootBufferInner {
            fragment_size,
            size,
            backing,
            claims: {
                let mut v = Vec::with_capacity(num_fragments);
                for _ in 0..num_fragments {
                    v.push(AtomicU8::new(FRAG_UNCLAIMED_MUTABLE));
                }
                v
            },
        };

        RootBuffer(Arc::new(inner))
    }

    pub fn num_fragments(&self) -> usize {
        self.0.fragment_size.needed_fragments(self.0.size)
    }

    /// Returns the number of sealed fragments.
    pub fn num_sealed(&self) -> usize {
        self.0.claims.iter().map(|v| {
            if v.load(Ordering::Relaxed) == FRAG_SEALED {
                1
            } else {
                0
            }
        }).sum()
    }

    pub fn as_ref_full<'a>(&'a self) -> Option<&'a [u8]> {
        if self.num_sealed() == self.num_fragments() {
            Some(unsafe { self.as_ref_full_unchecked() })
        } else {
            None
        }
    }

    pub unsafe fn as_ref_full_unchecked<'a>(&'a self) -> &'a [u8] {
        self.0.as_ref_full_unchecked()
    }

    pub fn claim(&self, fragment: usize) -> Option<FragmentBuffer> {
        debug_assert!(fragment < self.num_fragments());
        let state = &self.0.claims[fragment];

        let result = state.fetch_update(
            Ordering::Relaxed,
            Ordering::Relaxed,
            |val| {
                match val {
                    FRAG_UNCLAIMED_UNINITIALIZED => Some(FRAG_CLAIMED_UNINITIALIZED),
                    FRAG_UNCLAIMED_MUTABLE => Some(FRAG_CLAIMED_MUTABLE),
                    FRAG_SEALED => None,
                    FRAG_CLAIMED_UNINITIALIZED => None,
                    FRAG_CLAIMED_MUTABLE => None,
                    _ => unreachable!(),
                }
            },
        );

        let fragment_state = match result {
            Ok(FRAG_UNCLAIMED_UNINITIALIZED) => FragmentState::Uninitialized,
            Ok(FRAG_UNCLAIMED_MUTABLE) => FragmentState::Mutable,
            Ok(_) => unreachable!(),
            Err(FRAG_SEALED) => FragmentState::Sealed,
            Err(_) => return None,
        };

        Some(FragmentBuffer {
            state,
            container: self.0.clone(),
            buf: self.0.get_fragment_slice_ptr(fragment),
        })
    }
}

/// Buffer which is divided into N fragments of size S.
struct RootBufferInner {
    fragment_size: FragSize,
    size: usize,
    backing: *mut u8,
    claims: Vec<AtomicU8>,
}

unsafe impl Send for RootBufferInner {}
unsafe impl Sync for RootBufferInner {}

impl Drop for RootBufferInner {
    fn drop(&mut self) {
        let layout = std::alloc::Layout::from_size_align(
            self.size, self.fragment_size.size()).unwrap();
        unsafe {
            std::alloc::dealloc(self.backing, layout);
        }
    }
}

impl RootBufferInner {
    fn num_fragments(&self) -> usize {
        self.fragment_size.needed_fragments(self.size)
    }
    fn get_fragment_ptr(&self, fragment: usize) -> *mut u8 {
        debug_assert!(fragment < self.num_fragments());
        unsafe {
            self.backing.offset((self.fragment_size.size() * fragment) as isize)
        }
    }
    fn get_fragment_slice_ptr(&self, fragment: usize) -> *mut [u8] {
        let num_fragments = self.num_fragments();
        let base_fragment_size = self.fragment_size.size();
        let fragment_size = match fragment {
            f if f == num_fragments - 1 =>
                self.size % base_fragment_size,
            f if f < num_fragments =>
                base_fragment_size,
            _ => panic!(),
        };
        unsafe {
            let ptr = self.backing.offset(
                (fragment_size * fragment) as isize);
            std::ptr::slice_from_raw_parts_mut(ptr, fragment_size)
        }
    }
    unsafe fn as_ref_full_unchecked<'a>(&'a self) -> &'a [u8] {
        &*std::ptr::slice_from_raw_parts(self.backing, self.size)
    }
}

pub struct FragmentBuffer {
    container: Arc<RootBufferInner>,
    state: *const AtomicU8,
    buf: *mut [u8],
}

unsafe impl Send for FragmentBuffer {}
unsafe impl Sync for FragmentBuffer {}

impl std::fmt::Debug for FragmentBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.state() {
            FragmentState::Uninitialized => {
                f.debug_tuple("Uninitialized").finish()
            },
            FragmentState::Mutable => {
                f.debug_tuple("Mutable").field(&self.as_ref()).finish()
            },
            FragmentState::Sealed => {
                f.debug_tuple("Sealed").field(&self.as_ref()).finish()
            }
        }
    }
}
impl Drop for FragmentBuffer {
    fn drop(&mut self) {
        let new_state = match self.state() {
            FragmentState::Uninitialized => FRAG_UNCLAIMED_UNINITIALIZED,
            FragmentState::Mutable => FRAG_UNCLAIMED_MUTABLE,
            FragmentState::Sealed => FRAG_SEALED,
        };
        unsafe {
            (*self.state).store(new_state, Ordering::Relaxed);
        }
    }
}
impl FragmentBuffer {

    pub fn backing(&self) -> RootBuffer {
        RootBuffer(self.container.clone())
    }

    pub fn state(&self) -> FragmentState {
        let state_num = unsafe {
            (*self.state).load(Ordering::Relaxed)
        };
        match state_num {
            FRAG_CLAIMED_UNINITIALIZED => FragmentState::Uninitialized,
            FRAG_CLAIMED_MUTABLE => FragmentState::Mutable,
            FRAG_SEALED => FragmentState::Sealed,
            _ => unreachable!(),
        }
    }

    pub fn zero(&mut self) {
        assert!(self.state() != FragmentState::Sealed);
        unsafe {
            let buf = &mut *self.buf;
            std::ptr::write_bytes(&mut buf[0], 0, buf.len());
            (*self.state).store(FRAG_CLAIMED_MUTABLE, Ordering::Relaxed);
        }
    }

    pub fn fill(&mut self, data: &[u8]) {
        assert!(self.state() != FragmentState::Sealed);
        assert!(data.len() >= self.len());
        unsafe {
            std::ptr::copy_nonoverlapping(
                data.as_ptr(),
                (&mut *self.buf).as_mut_ptr(),
                self.len()
            )
        }
    }

    pub fn len(&self) -> usize {
        unsafe { (&*self.buf).len() }
    }

    pub fn as_ref<'a>(&'a self) -> &'a [u8] {
        assert!(self.state() != FragmentState::Uninitialized);
        unsafe {
            &*self.buf
        }
    }

    pub fn as_mut<'a>(&'a mut self) -> &'a mut [u8] {
        assert!(self.state() == FragmentState::Mutable);
        unsafe {
            &mut *self.buf
        }
    }

    pub fn seal(&mut self) {
        assert!(self.state() == FragmentState::Mutable);
        unsafe {
            (*self.state).store(FRAG_SEALED, Ordering::Relaxed);
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_usage() {
        let root = RootBuffer::new(1000, FragSize(8));

        {
            let mut frag1 = root.claim(0).unwrap();
            assert_eq!(frag1.state(), FragmentState::Uninitialized);
            assert!(root.claim(0).is_none());

            frag1.zero();
            assert_eq!(frag1.state(), FragmentState::Mutable);

            assert_eq!(frag1.as_ref().len(), 256);
        }

        let mut frag1 = root.claim(0).unwrap();
        assert_eq!(frag1.state(), FragmentState::Mutable);

        let mut frag2 = root.claim(1).unwrap();
        frag2.zero();
        assert_eq!(frag2.state(), FragmentState::Mutable);
        assert_eq!(frag2.as_ref().len(), 256);

        let mut frag3 = root.claim(2).unwrap();
        frag3.zero();
        assert_eq!(frag3.state(), FragmentState::Mutable);
        assert_eq!(frag3.as_ref().len(), 256);

        let mut frag4 = root.claim(3).unwrap();
        frag4.zero();
        assert_eq!(frag4.state(), FragmentState::Mutable);
        assert_eq!(frag4.as_ref().len(), 256 - ((256 * 4) - 1000));

        assert_eq!(root.num_sealed(), 0);
        frag4.seal();
        assert_eq!(root.num_sealed(), 1);

        let frag4_2 = root.claim(3).unwrap();
    }

}
