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
    max_heap_bytes: Option<usize>,
}

const STALE_SECS: u64 = 300; // 5 minutes

impl Tier2Pool {
    pub fn new(max_count: usize, pre_warm_count: usize, max_heap_bytes: Option<usize>) -> Self {
        let pool = Self {
            live_count: AtomicUsize::new(0),
            max_count,
            max_heap_bytes,
        };
        pool.pre_warm(pre_warm_count);
        pool
    }

    fn pre_warm(&self, count: usize) {
        for _ in 0..count {
            if !self.try_reserve_slot() {
                break;
            }
            match Tier2Session::new(self.max_heap_bytes) {
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
        let span = tracing::debug_span!(
            "pool.acquire",
            tier = "tier2",
            pool_size = self.max_count,
            waited_ms = tracing::field::Empty
        );
        let _guard = span.enter();
        let start = std::time::Instant::now();

        let session_result = (|| {
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

            match Tier2Session::new(self.max_heap_bytes) {
                Ok(session) => Ok(session),
                Err(err) => {
                    self.live_count.fetch_sub(1, Ordering::AcqRel);
                    Err(err)
                }
            }
        })();

        let waited_ms = start.elapsed().as_millis() as u64;
        tracing::Span::current().record("waited_ms", waited_ms);
        if waited_ms > 100 {
            tracing::warn!("Tier2 pool acquisition slow — pre-warm more runtimes");
        }

        session_result
    }

    /// Return a session to the thread-local pool.
    pub fn release_after_use(&self, session: Tier2Session) {
        session.destroy();
        self.live_count.fetch_sub(1, Ordering::Relaxed);

        // Pre-warm a replacement on the CURRENT thread.
        // This ensures the next acquire() on this thread is fast.
        if self.live_count.load(Ordering::Acquire) < self.max_count && self.try_reserve_slot() {
            match Tier2Session::new(self.max_heap_bytes) {
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

    /// Shut down the pool for the current thread and destroy all free sessions.
    pub fn shutdown(&self) {
        LOCAL_FREE.with(|free| {
            let mut free = free.borrow_mut();
            for pooled in free.drain(..) {
                pooled.session.destroy();
                self.live_count.fetch_sub(1, Ordering::Relaxed);
            }
        });
        tracing::debug!("Tier2Pool: shutdown on current thread completed");
    }
}

impl Drop for Tier2Pool {
    fn drop(&mut self) {
        // Clear sessions for the current thread
        self.shutdown();

        // Note: thread-local sessions on OTHER threads will be destroyed
        // when those threads exit. We can't easily clear them from here
        // without a global registry, but this prevents leaks on the
        // main/worker threads that explicitly shut down.
    }
}
