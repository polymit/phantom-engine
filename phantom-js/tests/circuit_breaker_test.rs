#![allow(clippy::unwrap_used, clippy::expect_used)]
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use phantom_js::circuit_breaker::{CLOSED, CircuitBreaker, HALF_OPEN, OPEN};

#[test]
fn starts_closed() {
    let breaker = CircuitBreaker::new();
    assert!(breaker.is_closed());
    assert_eq!(breaker.state(), CLOSED);
}

#[test]
fn opens_after_threshold() {
    let breaker = CircuitBreaker::with_config(5, 3, Duration::from_secs(30));
    for _ in 0..5 {
        breaker.record_failure();
    }
    assert_eq!(breaker.state(), OPEN);
}

#[test]
fn fails_fast_when_open() {
    let breaker = CircuitBreaker::with_config(5, 3, Duration::from_secs(30));
    for _ in 0..5 {
        breaker.record_failure();
    }
    assert_eq!(breaker.state(), OPEN);
    assert!(!breaker.is_permitted());
}

#[test]
fn transitions_to_half_open() {
    let breaker = CircuitBreaker::with_config(5, 3, Duration::from_secs(1));
    for _ in 0..5 {
        breaker.record_failure();
    }
    assert_eq!(breaker.state(), OPEN);

    thread::sleep(Duration::from_secs(2));
    assert!(breaker.is_permitted());
    assert_eq!(breaker.state(), HALF_OPEN);
}

#[test]
fn closes_after_3_successes() {
    let breaker = CircuitBreaker::with_config(5, 3, Duration::from_secs(1));
    for _ in 0..5 {
        breaker.record_failure();
    }
    thread::sleep(Duration::from_secs(2));
    assert!(breaker.is_permitted());
    assert_eq!(breaker.state(), HALF_OPEN);

    breaker.record_success();
    breaker.record_success();
    breaker.record_success();
    assert_eq!(breaker.state(), CLOSED);
}

#[test]
fn reopens_on_half_open_failure() {
    let breaker = CircuitBreaker::with_config(5, 3, Duration::from_secs(1));
    for _ in 0..5 {
        breaker.record_failure();
    }
    thread::sleep(Duration::from_secs(2));
    assert!(breaker.is_permitted());
    assert_eq!(breaker.state(), HALF_OPEN);

    breaker.record_failure();
    assert_eq!(breaker.state(), OPEN);
}

#[test]
fn resets_failure_count_on_success_in_closed() {
    let breaker = CircuitBreaker::with_config(5, 3, Duration::from_secs(30));
    for _ in 0..4 {
        breaker.record_failure();
    }
    assert_eq!(breaker.state(), CLOSED);

    breaker.record_success();

    for _ in 0..4 {
        breaker.record_failure();
    }
    assert_eq!(breaker.state(), CLOSED);

    breaker.record_failure();
    assert_eq!(breaker.state(), OPEN);
}

#[test]
fn concurrent_transitions_are_safe() {
    let breaker = Arc::new(CircuitBreaker::with_config(5, 3, Duration::from_millis(50)));
    let mut workers = Vec::new();

    for idx in 0..10 {
        let breaker = Arc::clone(&breaker);
        workers.push(thread::spawn(move || {
            for i in 0..2_000 {
                if breaker.is_permitted() {
                    if (i + idx) % 3 == 0 {
                        breaker.record_failure();
                    } else {
                        breaker.record_success();
                    }
                } else {
                    thread::sleep(Duration::from_millis(1));
                }
            }
        }));
    }

    for worker in workers {
        worker.join().expect("worker thread should not panic");
    }

    let state = breaker.state();
    assert!(matches!(state, CLOSED | OPEN | HALF_OPEN));
}
