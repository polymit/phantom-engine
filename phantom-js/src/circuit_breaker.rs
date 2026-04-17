use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const CLOSED: usize = 0;
pub const OPEN: usize = 1;
pub const HALF_OPEN: usize = 2;

/// Lock-free circuit breaker for runtime pool acquisition paths.
///
/// State transitions:
/// - Closed -> Open: failures reach threshold
/// - Open -> HalfOpen: probe interval elapsed and a probe is permitted
/// - HalfOpen -> Closed: consecutive successes reach threshold
/// - HalfOpen -> Open: any failure
#[derive(Debug)]
pub struct CircuitBreaker {
    state: AtomicUsize,
    failure_count: AtomicUsize,
    half_open_successes: AtomicUsize,
    opened_at_millis: AtomicU64,
    failure_threshold: usize,
    success_threshold: usize,
    probe_interval: Duration,
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new()
    }
}

impl CircuitBreaker {
    pub fn new() -> Self {
        Self::with_config(5, 3, Duration::from_secs(30))
    }

    pub fn with_config(
        failure_threshold: usize,
        success_threshold: usize,
        probe_interval: Duration,
    ) -> Self {
        Self {
            state: AtomicUsize::new(CLOSED),
            failure_count: AtomicUsize::new(0),
            half_open_successes: AtomicUsize::new(0),
            opened_at_millis: AtomicU64::new(0),
            failure_threshold: failure_threshold.max(1),
            success_threshold: success_threshold.max(1),
            probe_interval,
        }
    }

    pub fn state(&self) -> usize {
        self.state.load(Ordering::Acquire)
    }

    pub fn is_closed(&self) -> bool {
        self.state() == CLOSED
    }

    pub fn is_permitted(&self) -> bool {
        match self.state() {
            CLOSED => true,
            OPEN => self.try_enter_half_open(),
            HALF_OPEN => true,
            other => {
                tracing::warn!(state = other, "circuit breaker in unknown state");
                false
            }
        }
    }

    pub fn record_failure(&self) {
        match self.state() {
            CLOSED => {
                let failures = self.failure_count.fetch_add(1, Ordering::AcqRel) + 1;
                self.half_open_successes.store(0, Ordering::Release);
                if failures >= self.failure_threshold {
                    self.transition_to_open(CLOSED, "closed_failure_threshold_reached");
                }
            }
            HALF_OPEN => {
                self.transition_to_open(HALF_OPEN, "half_open_failure");
            }
            OPEN => {}
            other => {
                tracing::warn!(state = other, "record_failure ignored for unknown state");
            }
        }
    }

    pub fn record_success(&self) {
        match self.state() {
            CLOSED => {
                self.failure_count.store(0, Ordering::Release);
            }
            HALF_OPEN => {
                let successes = self.half_open_successes.fetch_add(1, Ordering::AcqRel) + 1;
                if successes >= self.success_threshold
                    && self
                        .state
                        .compare_exchange(HALF_OPEN, CLOSED, Ordering::AcqRel, Ordering::Acquire)
                        .is_ok()
                {
                    self.failure_count.store(0, Ordering::Release);
                    self.half_open_successes.store(0, Ordering::Release);
                    self.opened_at_millis.store(0, Ordering::Release);
                    tracing::info!(
                        success_threshold = self.success_threshold,
                        "circuit breaker closed"
                    );
                }
            }
            OPEN => {}
            other => {
                tracing::warn!(state = other, "record_success ignored for unknown state");
            }
        }
    }

    fn try_enter_half_open(&self) -> bool {
        let opened_at = self.opened_at_millis.load(Ordering::Acquire);
        let now = now_millis();
        if now.saturating_sub(opened_at) < probe_interval_millis(self.probe_interval) {
            return false;
        }

        match self
            .state
            .compare_exchange(OPEN, HALF_OPEN, Ordering::AcqRel, Ordering::Acquire)
        {
            Ok(_) => {
                self.half_open_successes.store(0, Ordering::Release);
                self.failure_count.store(0, Ordering::Release);
                tracing::info!(
                    probe_interval_ms = probe_interval_millis(self.probe_interval),
                    "circuit breaker half-open"
                );
                true
            }
            Err(current) => current == HALF_OPEN || current == CLOSED,
        }
    }

    fn transition_to_open(&self, from_state: usize, reason: &'static str) {
        if self
            .state
            .compare_exchange(from_state, OPEN, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            self.opened_at_millis.store(now_millis(), Ordering::Release);
            self.half_open_successes.store(0, Ordering::Release);
            self.failure_count.store(0, Ordering::Release);
            tracing::warn!(
                from_state,
                reason,
                failure_threshold = self.failure_threshold,
                "circuit breaker opened"
            );
        }
    }
}

fn now_millis() -> u64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(d) => {
            let millis = d.as_millis();
            if millis > u64::MAX as u128 {
                u64::MAX
            } else {
                millis as u64
            }
        }
        Err(_) => 0,
    }
}

fn probe_interval_millis(duration: Duration) -> u64 {
    let millis = duration.as_millis();
    if millis > u64::MAX as u128 {
        u64::MAX
    } else {
        millis as u64
    }
}
