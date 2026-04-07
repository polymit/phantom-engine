use crossbeam::queue::SegQueue;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use crate::error::PhantomJsError;
use crate::tier2::session::Tier2Session;

struct PooledSession {
    session: Tier2Session,
    // V8 isolates held idle past 5 minutes risk their snapshot state going
    // stale relative to persona rotation. Evict and replace proactively.
    created_at: Instant,
}

pub struct Tier2Pool {
    free: SegQueue<PooledSession>,
    live_count: AtomicUsize,
    max_count: usize,
}

const STALE_SECS: u64 = 300; // 5 minutes — matches blueprint spec

impl Tier2Pool {
    /// Create a pool and synchronously pre-warm `pre_warm_count` V8 sessions.
    ///
    /// Tier2Session::new() is synchronous (snapshot load via JsRuntime::new),
    /// so pre_warm runs on the calling thread — call this before spawning tasks.
    /// Blueprint specifies 10 pre-warmed sessions.
    pub fn new(max_count: usize, pre_warm_count: usize) -> Self {
        let pool = Self {
            free: SegQueue::new(),
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
                Ok(session) => {
                    self.free.push(PooledSession {
                        session,
                        created_at: Instant::now(),
                    });
                }
                Err(e) => {
                    self.live_count.fetch_sub(1, Ordering::AcqRel);
                    tracing::warn!("Tier2Pool pre-warm skipped: {:?}", e);
                }
            }
        }
    }

    /// Check out a session. Returns a fresh one if the free queue is empty.
    ///
    /// Stale sessions (idle > 5 min) are evicted — their persona may be
    /// out-of-date and V8's idle GC may have compacted the heap in ways
    /// that affect timing fingerprints.
    pub fn acquire(&self) -> Result<Tier2Session, PhantomJsError> {
        while let Some(pooled) = self.free.pop() {
            if pooled.created_at.elapsed().as_secs() < STALE_SECS {
                return Ok(pooled.session);
            }
            pooled.session.destroy();
            self.live_count.fetch_sub(1, Ordering::Relaxed);
        }

        if !self.try_reserve_slot() {
            return Err(PhantomJsError::PoolExhausted {
                max: self.max_count,
            });
        }

        let session = match Tier2Session::new() {
            Ok(session) => session,
            Err(err) => {
                self.live_count.fetch_sub(1, Ordering::AcqRel);
                return Err(err);
            }
        };
        Ok(session)
    }

    /// Return a session after use.
    ///
    /// Per D-40: the V8 global environment is polluted after page JS runs —
    /// event listeners, patched prototypes, and framework state cannot be
    /// cleaned up without a full isolate teardown. Destroy immediately,
    /// pre-warm a replacement in-place (synchronous, cheap — snapshot load).
    pub fn release_after_use(&self, session: Tier2Session) {
        session.destroy();
        self.live_count.fetch_sub(1, Ordering::Relaxed);

        // Replacement is synchronous for Tier 2 — JsRuntime::new from snapshot
        // is fast enough (<5ms) that blocking here is acceptable.
        // We avoid spawning a Tokio task to keep the pool usable from
        // non-async call sites (e.g., sync MCP tool handlers).
        self.pre_warm(1);
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
        let mut sessions = Vec::new();
        while let Some(pooled) = self.free.pop() {
            sessions.push(pooled);
        }
        // SegQueue is FIFO. sessions[0] is the oldest isolate.
        // V8 requires reverse-order drop (LIFO).
        sessions.reverse();
        // Vec drop will now destroy sessions in newer-to-older order.
    }
}

// SAFETY: Tier2Pool manages thread-bound Tier2Sessions (V8). The pool's internal
// state (SegQueue and atomics) is thread-safe.
unsafe impl Send for Tier2Pool {}
unsafe impl Sync for Tier2Pool {}
