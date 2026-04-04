use crossbeam::queue::SegQueue;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use crate::error::PhantomJsError;
use crate::tier1::session::Tier1Session;

struct PooledSession {
    session: Tier1Session,
    // Used to evict sessions that sat idle past the 5-minute staleness window.
    // A session that has been idle for 5+ minutes may have had its QuickJS
    // interrupt timer fire, leaving the isolate in a killed state.
    created_at: Instant,
}

pub struct Tier1Pool {
    free: SegQueue<PooledSession>,
    // Tracks every session currently alive — both in `free` and checked-out.
    // This lets acquire() enforce the hard cap without draining the queue.
    live_count: AtomicUsize,
    max_count: usize,
}

const STALE_SECS: u64 = 300; // 5 minutes — matches blueprint spec

impl Tier1Pool {
    /// Create a pool and pre-warm `pre_warm_count` sessions immediately.
    /// Blueprint specifies 10 pre-warmed sessions at startup.
    pub async fn new(max_count: usize, pre_warm_count: usize) -> Self {
        let pool = Self {
            free: SegQueue::new(),
            live_count: AtomicUsize::new(0),
            max_count,
        };
        pool.pre_warm(pre_warm_count).await;
        pool
    }

    /// Fills the pool up to `count` additional sessions.
    async fn pre_warm(&self, count: usize) {
        for _ in 0..count {
            if self.live_count.load(Ordering::Relaxed) >= self.max_count {
                break;
            }
            match Tier1Session::new().await {
                Ok(session) => {
                    self.free.push(PooledSession {
                        session,
                        created_at: Instant::now(),
                    });
                    self.live_count.fetch_add(1, Ordering::Relaxed);
                }
                Err(e) => {
                    tracing::warn!("Tier1Pool pre-warm skipped: {:?}", e);
                }
            }
        }
    }

    /// Check out a session. Returns a fresh one if the free queue is empty.
    ///
    /// Stale sessions (idle > 5 min) are discarded and replaced rather than
    /// returned — once the QuickJS interrupt timer fires the isolate is dead.
    pub async fn acquire(&self) -> Result<Tier1Session, PhantomJsError> {
        // Drain stale entries from the front of the queue before handing one out.
        while let Some(pooled) = self.free.pop() {
            if pooled.created_at.elapsed().as_secs() < STALE_SECS {
                return Ok(pooled.session);
            }
            // Stale — destroy and account for the freed slot
            pooled.session.destroy();
            self.live_count.fetch_sub(1, Ordering::Relaxed);
        }

        // Pool miss — spin up a fresh session if capacity allows
        if self.live_count.load(Ordering::Relaxed) >= self.max_count {
            return Err(PhantomJsError::PoolExhausted { max: self.max_count });
        }

        let session = Tier1Session::new().await?;
        self.live_count.fetch_add(1, Ordering::Relaxed);
        Ok(session)
    }

    /// Return a session after use.
    ///
    /// Per D-40: post-execution globals are polluted. Recycling a used session
    /// would leak DOM handles and event listeners across session boundaries,
    /// creating a detectable anomaly and a correctness bug. We destroy it
    /// immediately and spawn a background task to refill the pool slot.
    pub fn release_after_use(&self, session: Tier1Session) {
        session.destroy();
        self.live_count.fetch_sub(1, Ordering::Relaxed);

        // Cast to usize before spawning — usize is Send, raw pointer is not.
        // SAFETY: pool is held in Arc<Tier1Pool> by the caller and outlives
        // all tasks it spawns. The address is stable because Arc does not move
        // its inner allocation. We never take &mut through this path.
        let addr = self as *const Self as usize;
        tokio::spawn(async move {
            let pool = unsafe { &*(addr as *const Tier1Pool) };
            pool.pre_warm(1).await;
        });
    }
}
