use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::error::PhantomJsError;
use crate::tier1::session::Tier1Session;

pub struct Tier1Pool {
    // Tracks every checked-out session currently alive.
    // acquire() reserves a slot before constructing Tier1Session.
    live_count: AtomicUsize,
    max_count: usize,
}

impl Tier1Pool {
    /// Create a pool and pre-warm `pre_warm_count` sessions immediately.
    /// Pre-warm is a one-time constructor warm-up only.
    pub async fn new(max_count: usize, pre_warm_count: usize) -> Arc<Self> {
        let pool = Arc::new(Self {
            live_count: AtomicUsize::new(0),
            max_count,
        });
        pool.pre_warm(pre_warm_count).await;
        pool
    }

    /// Warm up runtime internals by constructing and immediately destroying
    /// temporary sessions on the current task thread.
    ///
    /// We intentionally do not retain pre-warmed Tier1Session instances in a
    /// cross-thread queue because QuickJS runtime objects are thread-affine.
    async fn pre_warm(&self, count: usize) {
        for _ in 0..count.min(self.max_count) {
            match Tier1Session::new().await {
                Ok(session) => session.destroy(),
                Err(e) => tracing::warn!("Tier1Pool pre-warm skipped: {:?}", e),
            }
        }
    }

    /// Check out a fresh Tier1Session.
    pub async fn acquire(&self) -> Result<Tier1Session, PhantomJsError> {
        if !self.try_reserve_slot() {
            return Err(PhantomJsError::PoolExhausted {
                max: self.max_count,
            });
        }

        let session = match Tier1Session::new().await {
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
    /// Per D-40: post-execution globals are polluted. Recycling a used session
    /// would leak DOM handles and event listeners across session boundaries,
    /// creating a detectable anomaly and a correctness bug. We destroy it
    /// immediately. The next acquire call lazily fills capacity back up.
    pub fn release_after_use(self: &Arc<Self>, session: Tier1Session) {
        session.destroy();
        self.live_count.fetch_sub(1, Ordering::Relaxed);
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
