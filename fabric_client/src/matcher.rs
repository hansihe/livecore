use std::sync::Arc;
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Mutex;

use tokio::time::{timeout_at, Instant};
use tokio::sync::oneshot;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum HoldSuccess {
    // There was a waiting receiver that immediately received the value.
    MatchedWaiting,
    // A receiver claimed the item after a period of waiting.
    MatchedAfter,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum HoldError {
    // Another item replaced this one.
    Replaced,
    // Timed out without being claimed.
    UnclaimedTimeout,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PermitError {
    // There was already a waiter for the given key.
    AlreadyWaiting,
    Timeout,
}

enum SlotState<V> {
    Holding(oneshot::Sender<bool>, V),
    Waiting(oneshot::Sender<V>),
}

pub struct Matcher<K, V> {
    slots: Mutex<HashMap<K, SlotState<V>>>,
}

impl<K, V> Matcher<K, V> {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            slots: Mutex::new(HashMap::new()),
        })
    }

    pub fn len(&self) -> usize {
        let mut inner = self.slots.lock().unwrap();
        inner.len()
    }
}

impl<K: Hash + Eq + Clone, V> Matcher<K, V> {

    pub async fn send(&self, key: &K, timeout_time: Instant, conn: V) -> Result<HoldSuccess, HoldError> {
        let receiver;

        {
            let mut inner = self.slots.lock().unwrap();

            match inner.remove(key) {
                Some(SlotState::Waiting(sender)) => {
                    // The error case only happens if the receiver of the
                    // channel was dropped.
                    // This should never happen, as the other end will
                    // lock the map and remove the sender entry before
                    // dropping the receiver.
                    sender.send(conn)
                          .map_err(|_| ())
                          .expect("other end of channel should never have been dropped");
                    return Ok(HoldSuccess::MatchedWaiting);
                }
                Some(SlotState::Holding(sender, _)) => {
                    // There was already a value held for the value,
                    // but we want to simply replace it.
                    // Notify the waiting holder that it has been replaced.
                    sender.send(true)
                        .expect("other end of channel should never have been dropped");
                }
                None => ()
            }

            let (sender, recv) = oneshot::channel();
            inner.insert(key.clone(), SlotState::Holding(sender, conn));
            receiver = recv;
        }

        match timeout_at(timeout_time, receiver).await {
            Ok(Ok(true)) => {
                // Another item was posted with the key, we have been replaced.
                // Return error immediately.
                return Err(HoldError::Replaced);
            }
            Ok(Ok(false)) => {
                // The item was claimed by a receiver.
                // Return success immediately.
                return Ok(HoldSuccess::MatchedAfter);
            }
            Ok(Err(_)) => {
                // Other side of the channel should never be dropped.
                unreachable!()
            }
            Err(_) => {
                // We got a timeout, but this does not mean the item hasn't been
                // claimed in the meantime.
                //
                // Take a lock on the map and check if the item was claimed.
                //
                // There is a potential race condition here if a nonce is used
                // several times, but the nonces are supposed to be long and
                // random, and nothing bad will happen, so this is acceptible.
                let mut inner = self.slots.lock().unwrap();

                match inner.remove(key) {
                    Some(SlotState::Waiting(_)) => unreachable!(),
                    Some(SlotState::Holding(_, _)) => {
                        return Err(HoldError::UnclaimedTimeout);
                    }
                    None => {
                        // Item was claimed in the timeframe between the timeout
                        // and when we got the lock. This is also a success.
                        return Ok(HoldSuccess::MatchedAfter);
                    }
                }
            }
        }

    }

    pub async fn receive(&self, key: &K, timeout_time: Instant) -> Result<V, PermitError> {
        let receiver;

        {
            let mut inner = self.slots.lock().unwrap();

            match inner.remove(key) {
                Some(val @ SlotState::Waiting(_)) => {
                    inner.insert(key.clone(), val);
                    return Err(PermitError::AlreadyWaiting);
                }
                Some(SlotState::Holding(sender, value)) => {
                    sender.send(false).unwrap();
                    return Ok(value);
                }
                None => (),
            }

            let (sender, recv) = oneshot::channel();
            receiver = recv;
            inner.insert(key.clone(), SlotState::Waiting(sender));
        }

        let result;

        match timeout_at(timeout_time, receiver).await {
            Ok(Ok(value)) => {
                result = Ok(value);
            },
            Ok(Err(_)) => {
                unreachable!()
            }
            Err(_) => {
                result = Err(PermitError::Timeout);
            }
        }

        let mut inner = self.slots.lock().unwrap();
        inner.remove(key);

        result
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use futures::task::Poll;
    use futures::future::FutureExt;

    use tokio::sync::Barrier;
    use tokio::spawn;

    use super::{Matcher, HoldSuccess, HoldError};

    #[tokio::test]
    async fn send_before_receive() {
        let timeout_time = tokio::time::Instant::now() + std::time::Duration::from_millis(10000);

        let m1 = Arc::new(Matcher::<u32, u32>::new());
        let m2 = m1.clone();

        let s1 = m1.send(&12, timeout_time, 123).fuse();
        tokio::pin!(s1);

        // Poll once so the matcher state gets updated
        assert!(futures::poll!(&mut s1) == Poll::Pending);

        let res2 = m2.receive(&12, timeout_time).await;
        assert!(res2 == Ok(123));

        let res1 = s1.await;
        assert!(res1 == Ok(HoldSuccess::MatchedAfter));

        assert!(m1.len() == 0);
    }

    #[tokio::test]
    async fn receive_before_send() {
        let timeout_time = tokio::time::Instant::now() + std::time::Duration::from_millis(10000);

        let m1 = Arc::new(Matcher::<u32, u32>::new());
        let m2 = m1.clone();

        let s2 = m2.receive(&12, timeout_time).fuse();
        tokio::pin!(s2);

        // Poll once so the matcher state gets updated
        assert!(futures::poll!(&mut s2) == Poll::Pending);

        let res1 = m1.send(&12, timeout_time, 123).await;
        assert!(res1 == Ok(HoldSuccess::MatchedWaiting));

        let res2 = s2.await;
        assert!(res2 == Ok(123));

        assert!(m1.len() == 0);
    }

    #[tokio::test]
    async fn duplicate_send() {
        // Sending twice on the same key should result in the first send being
        // replaced, and an error reported.
        // The second send should proceed as normal.
        let timeout_time = tokio::time::Instant::now() + std::time::Duration::from_millis(10000);

        let m1 = Arc::new(Matcher::<u32, u32>::new());
        let m2 = m1.clone();
        let m3 = m1.clone();

        let s1 = m1.send(&12, timeout_time, 1).fuse();
        tokio::pin!(s1);

        // Poll once so the matcher state gets updated
        assert!(futures::poll!(&mut s1) == Poll::Pending);

        let s2 = m2.send(&12, timeout_time, 2).fuse();
        tokio::pin!(s2);

        // Poll once so the matcher state gets updated
        assert!(futures::poll!(&mut s2) == Poll::Pending);

        let res = s1.await;
        assert!(res == Err(HoldError::Replaced));

        assert!(m1.len() == 1);

        let r1 = m3.receive(&12, timeout_time);
        let res = r1.await;
        assert!(res == Ok(2));

        assert!(m1.len() == 0);

        let res = s2.await;
        assert!(res == Ok(HoldSuccess::MatchedAfter));

        assert!(m1.len() == 0);
    }

}
