use std::cell::RefCell;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use crate::error::PhantomJsError;
use crate::tier2::session::Tier2Session;

struct PooledSession {
    session: Tier2Session,
    created_at: Instant,
}

thread_local! {
    static LOCAL_FREE: RefCell<Vec<PooledSession>> = const { RefCell::new(Vec::new()) };
}

pub struct Tier2Pool {
    live_count: AtomicUsize,
    max_count: usize,
}

const STALE_SECS: u64 = 300; // 5 minutes

impl Tier2Pool {
    pub fn new(max_count: usize, pre_warm_count: usize) -> Self {
        let pool = Self {
            live_count: AtomicUsize::new(0),
            max_count,
        };
        pool.pre_warm(pre_warm_count);
        pool
    }

    fn pre_warm(&self, count: usize) {
        for _ in 0..count {
            if !self.try_reserve_slot() {
                break;
            }
            match Tier2Session::new() {
                Ok(new_session) => {
                    LOCAL_FREE.with(|free| {
                        free.borrow_mut().push(PooledSession {
                            session: new_session,
                            created_at: Instant::now(),
                        });
                    });
                }
                Err(_) => {
                    self.live_count.fetch_sub(1, Ordering::AcqRel);
                    break;
                }
            }
        }
    }

    /// Check out a session from the thread-local pool.
    pub fn acquire(&self) -> Result<Tier2Session, PhantomJsError> {
        // 1. Check thread-local free list
        let pooled = LOCAL_FREE.with(|free| {
            let mut free = free.borrow_mut();
            while let Some(pooled) = free.pop() {
                if pooled.created_at.elapsed().as_secs() < STALE_SECS {
                    return Some(pooled.session);
                }
                // Stale
                pooled.session.destroy();
                self.live_count.fetch_sub(1, Ordering::Relaxed);
            }
            None
        });

        if let Some(session) = pooled {
            return Ok(session);
        }

        // 2. Try to create a new one if limit not reached
        if !self.try_reserve_slot() {
            return Err(PhantomJsError::PoolExhausted {
                max: self.max_count,
            });
        }

        match Tier2Session::new() {
            Ok(session) => Ok(session),
            Err(err) => {
                self.live_count.fetch_sub(1, Ordering::AcqRel);
                Err(err)
            }
        }
    }

    /// Return a session to the thread-local pool.
    pub fn release_after_use(&self, session: Tier2Session) {
        session.destroy();
        self.live_count.fetch_sub(1, Ordering::Relaxed);

        // Pre-warm a replacement on the CURRENT thread.
        // This ensures the next acquire() on this thread is fast.
        if self.live_count.load(Ordering::Acquire) < self.max_count && self.try_reserve_slot() {
            match Tier2Session::new() {
                Ok(new_session) => {
                    LOCAL_FREE.with(|free| {
                        free.borrow_mut().push(PooledSession {
                            session: new_session,
                            created_at: Instant::now(),
                        });
                    });
                }
                Err(_) => {
                    self.live_count.fetch_sub(1, Ordering::AcqRel);
                }
            }
        }
    }

    fn try_reserve_slot(&self) -> bool {
        let mut cur = self.live_count.load(Ordering::Acquire);
        loop {
            if cur >= self.max_count {
                return false;
            }
            match self.live_count.compare_exchange_weak(
                cur,
                cur + 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return true,
                Err(next) => cur = next,
            }
        }
    }
}

impl Drop for Tier2Pool {
    fn drop(&mut self) {
        // Note: thread-local sessions will be destroyed when the threads exit.
        // We can't easily clear all of them from here, but this is acceptable
        // since the pool itself is dropping and no new sessions can be acquired.
    }
}
