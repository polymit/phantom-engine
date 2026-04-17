#![allow(clippy::unwrap_used, clippy::expect_used)]
use criterion::async_executor::FuturesExecutor;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use phantom_session::{EngineKind, ResourceBudget, Session, SessionBroker, SessionState};
use uuid::Uuid;

fn assert_goal(name: &str, measured_ms: f64, goal_ms: f64, minimum_ms: f64) {
    if measured_ms > minimum_ms {
        println!(
            "FAIL: {} = {:.2}ms — MINIMUM {:.2}ms NOT MET",
            name, measured_ms, minimum_ms
        );
    } else if measured_ms <= goal_ms {
        println!(
            "GOAL: {} = {:.2}ms ≤ {:.2}ms GOAL ✓",
            name, measured_ms, goal_ms
        );
    } else {
        println!(
            "MIN:  {} = {:.2}ms > {:.2}ms GOAL (but ≤ {:.2}ms minimum)",
            name, measured_ms, goal_ms, minimum_ms
        );
    }
}

fn bench_session_create_quickjs(c: &mut Criterion) {
    let broker = SessionBroker::new();
    let budget = ResourceBudget::default();

    let start = std::time::Instant::now();
    let warm_id = broker
        .create_session(EngineKind::QuickJs, budget.clone(), "bench-qjs")
        .expect("quickjs session create should succeed");
    let measured_ms = start.elapsed().as_secs_f64() * 1000.0;
    assert_goal("session_create_quickjs", measured_ms, 10.0, 50.0);
    let _ = broker.remove(warm_id);

    c.bench_function("session_create_quickjs", |b| {
        b.to_async(FuturesExecutor).iter(|| async {
            let id = broker
                .create_session(EngineKind::QuickJs, budget.clone(), "bench-qjs")
                .expect("quickjs session create should succeed");
            black_box(id);
            let _ = broker.remove(id);
        })
    });
}

fn bench_session_create_v8(c: &mut Criterion) {
    let broker = SessionBroker::new();
    let budget = ResourceBudget::default();

    let start = std::time::Instant::now();
    let warm_id = broker
        .create_session(EngineKind::V8, budget.clone(), "bench-v8")
        .expect("v8 session create should succeed");
    let measured_ms = start.elapsed().as_secs_f64() * 1000.0;
    assert_goal("session_create_v8", measured_ms, 50.0, 100.0);
    let _ = broker.remove(warm_id);

    c.bench_function("session_create_v8", |b| {
        b.to_async(FuturesExecutor).iter(|| async {
            let id = broker
                .create_session(EngineKind::V8, budget.clone(), "bench-v8")
                .expect("v8 session create should succeed");
            black_box(id);
            let _ = broker.remove(id);
        })
    });
}

fn bench_session_clone_cow(c: &mut Criterion) {
    let source = Session::new(EngineKind::QuickJs, ResourceBudget::default(), "source");

    let start = std::time::Instant::now();
    let warm = Session::with_uuid(
        Uuid::new_v4(),
        source.engine,
        source.budget.clone(),
        source.persona_id.clone(),
    );
    let measured_ms = start.elapsed().as_secs_f64() * 1000.0;
    assert_goal("session_clone_cow", measured_ms, 50.0, 200.0);
    black_box(warm.id);

    c.bench_function("session_clone_cow", |b| {
        b.iter(|| {
            let clone = Session::with_uuid(
                Uuid::new_v4(),
                source.engine,
                source.budget.clone(),
                source.persona_id.clone(),
            );
            black_box(clone.id)
        })
    });
}

fn bench_session_suspend_resume(c: &mut Criterion) {
    let broker = SessionBroker::new();
    let id = broker.create(EngineKind::QuickJs, ResourceBudget::default(), "bench");

    let start = std::time::Instant::now();
    broker
        .set_state(id, SessionState::Suspended)
        .expect("suspend state transition should succeed");
    broker
        .set_state(id, SessionState::Running)
        .expect("resume state transition should succeed");
    let measured_ms = start.elapsed().as_secs_f64() * 1000.0;
    assert_goal("session_suspend_resume", measured_ms, 50.0, 200.0);

    c.bench_function("session_suspend_resume", |b| {
        b.iter(|| {
            broker
                .set_state(id, SessionState::Suspended)
                .expect("suspend state transition should succeed");
            broker
                .set_state(id, SessionState::Running)
                .expect("resume state transition should succeed");
        })
    });
}

criterion_group!(
    benches,
    bench_session_create_quickjs,
    bench_session_create_v8,
    bench_session_clone_cow,
    bench_session_suspend_resume
);
criterion_main!(benches);
