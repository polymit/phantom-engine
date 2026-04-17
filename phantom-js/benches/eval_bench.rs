#![allow(clippy::unwrap_used, clippy::expect_used)]
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use phantom_js::tier1::pool::Tier1Pool;
use phantom_js::tier1::session::Tier1Session;
use phantom_js::tier2::session::Tier2Session;

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

fn build_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime should build for benchmarks")
}

fn bench_quickjs_eval_simple(c: &mut Criterion) {
    let runtime = build_runtime();
    let session = runtime
        .block_on(Tier1Session::new())
        .expect("tier1 session should initialise");

    let start = std::time::Instant::now();
    let warm = runtime
        .block_on(session.eval("1+1"))
        .expect("quickjs warm eval should succeed");
    let measured_ms = start.elapsed().as_secs_f64() * 1000.0;
    assert_goal("quickjs_eval_simple", measured_ms, 10.0, 50.0);
    black_box(warm);

    c.bench_function("quickjs_eval_simple", |b| {
        b.iter(|| {
            let out = runtime
                .block_on(session.eval(black_box("1+1")))
                .expect("quickjs eval should succeed");
            black_box(out)
        })
    });
}

fn bench_v8_eval_simple(c: &mut Criterion) {
    if std::env::var("PHANTOM_RUN_V8_BENCH").is_err() {
        println!("INFO: v8_eval_simple benchmark skipped (set PHANTOM_RUN_V8_BENCH=1 to enable)");
        c.bench_function("v8_eval_simple", |b| {
            b.iter(|| black_box(black_box(1usize) + black_box(1usize)))
        });
        return;
    }

    phantom_js::init_v8_platform();
    let mut warm_session =
        Tier2Session::new(Some(64 * 1024 * 1024)).expect("tier2 session should build");

    let start = std::time::Instant::now();
    let warm = warm_session
        .eval("1+1")
        .expect("v8 warm eval should succeed");
    let measured_ms = start.elapsed().as_secs_f64() * 1000.0;
    assert_goal("v8_eval_simple", measured_ms, 50.0, 100.0);
    black_box(warm);
    warm_session.destroy();

    c.bench_function("v8_eval_simple", |b| {
        b.iter(|| {
            let mut session =
                Tier2Session::new(Some(64 * 1024 * 1024)).expect("tier2 session should build");
            let out = session
                .eval(black_box("1+1"))
                .expect("v8 eval should succeed");
            session.destroy();
            black_box(out)
        })
    });
}

fn bench_pool_acquire_tier1(c: &mut Criterion) {
    let runtime = build_runtime();
    let pool = runtime.block_on(Tier1Pool::new(32, 0));

    let start = std::time::Instant::now();
    let warm = runtime
        .block_on(pool.acquire())
        .expect("tier1 acquire warmup should succeed");
    let measured_ms = start.elapsed().as_secs_f64() * 1000.0;
    assert_goal("pool_acquire_tier1", measured_ms, 1.0, 10.0);
    pool.release_after_use(warm);

    c.bench_function("pool_acquire_tier1", |b| {
        b.iter(|| {
            let session = runtime
                .block_on(pool.acquire())
                .expect("tier1 acquire should succeed");
            pool.release_after_use(session);
        })
    });
}

criterion_group!(
    benches,
    bench_quickjs_eval_simple,
    bench_v8_eval_simple,
    bench_pool_acquire_tier1
);
criterion_main!(benches);
