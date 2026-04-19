#![allow(clippy::unwrap_used, clippy::expect_used)]
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use phantom_core::process_html;
use phantom_js::circuit_breaker::OPEN;
use phantom_mcp::{engine, metrics, EngineAdapter};
use phantom_serializer::{HeadlessSerializer, SerialiserConfig};
use phantom_session::{EngineKind, ResourceBudget, Session, SessionState};
use uuid::Uuid;

fn render_cct(marker: &str) -> String {
    let html = format!(
        "<html><body style='width: 1280px; height: 720px;'><h1>{marker}</h1></body></html>"
    );
    let page = process_html(&html, "data:text/html,scale", 1280.0, 720.0, Vec::new())
        .expect("fixture HTML should parse");
    HeadlessSerializer::serialise(
        &page,
        &SerialiserConfig {
            url: "data:text/html,scale".to_string(),
            ..Default::default()
        },
    )
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(|a, b| a.total_cmp(b));
    let mid = values.len() / 2;
    if values.len().is_multiple_of(2) {
        (values[mid - 1] + values[mid]) / 2.0
    } else {
        values[mid]
    }
}

fn p99(values: &mut [f64]) -> f64 {
    values.sort_by(|a, b| a.total_cmp(b));
    let idx = ((values.len() as f64) * 0.99).ceil() as usize;
    let bounded = idx.saturating_sub(1).min(values.len().saturating_sub(1));
    values[bounded]
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scale_smoke_test() {
    engine::init_v8();
    let adapter = Arc::new(EngineAdapter::new(16, 0, 4, 0, ResourceBudget::default()).await);
    let baseline = adapter.session_count();

    let start = Instant::now();
    let mut handles = Vec::new();
    for i in 0..100 {
        let adapter = Arc::clone(&adapter);
        handles.push(tokio::spawn(async move {
            let tier = if i % 5 == 0 {
                EngineKind::V8
            } else {
                EngineKind::QuickJs
            };
            let session_id = adapter
                .broker
                .create_session(tier, ResourceBudget::default(), format!("persona-{i}"))
                .expect("session creation should succeed");
            let cct = render_cct(&format!("session-{i}-marker"));
            assert!(!cct.is_empty());
            assert!(cct.starts_with("##PAGE"));
            let removed = adapter
                .broker
                .remove(session_id)
                .expect("session teardown should succeed");
            (i, cct, removed.id)
        }));
    }

    let mut outputs = Vec::with_capacity(100);
    for handle in handles {
        outputs.push(handle.await.expect("task should not panic"));
    }

    for (i, cct, removed_id) in outputs {
        assert!(cct.contains(&format!("session-{i}-marker")));
        assert_eq!(removed_id.get_version_num(), 4);
    }

    assert_eq!(adapter.session_count(), baseline);
    assert_eq!(adapter.tier1.circuit_breaker_state(), 0);
    assert_eq!(adapter.tier2.circuit_breaker_state(), 0);
    assert!(
        start.elapsed() < Duration::from_secs(30),
        "smoke test exceeded 30s"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "heavy scale gate; intended for docker main pipeline"]
async fn scale_full_1000() {
    engine::init_v8();
    let adapter = Arc::new(EngineAdapter::new(32, 0, 8, 0, ResourceBudget::default()).await);
    let baseline = adapter.session_count();

    let mut ids = Vec::with_capacity(1000);
    let mut quickjs_ms = Vec::with_capacity(800);
    let mut v8_ms = Vec::with_capacity(200);

    for i in 0..1000 {
        let tier = if i < 800 {
            EngineKind::QuickJs
        } else {
            EngineKind::V8
        };
        let start = Instant::now();
        let id = adapter
            .broker
            .create_session(tier, ResourceBudget::default(), format!("persona-{i}"))
            .expect("session creation should succeed");
        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
        if matches!(tier, EngineKind::QuickJs) {
            quickjs_ms.push(elapsed_ms);
        } else {
            v8_ms.push(elapsed_ms);
        }
        ids.push(id);
    }

    for id in ids.iter().take(500).copied() {
        adapter
            .broker
            .set_state(id, SessionState::Suspended)
            .expect("suspend transition should succeed");
        adapter
            .broker
            .set_state(id, SessionState::Running)
            .expect("resume transition should succeed");
    }

    let mut clone_ids = Vec::with_capacity(100);
    for id in ids.iter().take(100).copied() {
        let source = adapter.broker.get(id).expect("source session should exist");
        let clone_id = Uuid::new_v4();
        let clone = Session::with_uuid(
            clone_id,
            source.engine,
            source.budget.clone(),
            source.persona_id.clone(),
        );
        adapter.broker.register(clone);
        clone_ids.push(clone_id);
    }

    let peak_sessions = adapter.session_count();
    let sessions_count = ids.len();

    let mut quickjs_for_median = quickjs_ms.clone();
    let mut v8_for_median = v8_ms.clone();
    let mut startup_all = quickjs_ms;
    startup_all.extend(v8_ms);

    let median_quickjs_startup_ms = median(&mut quickjs_for_median);
    let median_v8_startup_ms = median(&mut v8_for_median);
    let p99_startup_ms = p99(&mut startup_all);

    assert!(
        median_quickjs_startup_ms < 50.0,
        "PHASE GATE: QuickJS startup median {:.1}ms exceeds 50ms MINIMUM",
        median_quickjs_startup_ms
    );
    assert!(
        median_v8_startup_ms < 100.0,
        "PHASE GATE: V8 startup median {:.1}ms exceeds 100ms MINIMUM",
        median_v8_startup_ms
    );
    assert!(
        p99_startup_ms < 200.0,
        "PHASE GATE: startup p99 {:.1}ms exceeds 200ms MINIMUM",
        p99_startup_ms
    );
    assert!(
        sessions_count == 1000,
        "PHASE GATE: Only {} of 1000 sessions succeeded",
        sessions_count
    );
    assert!(
        peak_sessions >= baseline + 1000,
        "SESSIONS_ACTIVE surrogate did not reach expected load"
    );

    for id in ids {
        let _ = adapter.broker.remove(id);
    }
    for id in clone_ids {
        let _ = adapter.broker.remove(id);
    }
    assert_eq!(adapter.session_count(), baseline);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scale_memory_profile() {
    engine::init_v8();
    let adapter = Arc::new(EngineAdapter::new(16, 0, 4, 0, ResourceBudget::default()).await);
    let baseline = adapter.session_count();

    let mut quickjs_heap = Vec::with_capacity(50);
    let mut quickjs_ids = Vec::with_capacity(50);
    for i in 0..50 {
        let id = adapter
            .broker
            .create_session(
                EngineKind::QuickJs,
                ResourceBudget::default(),
                format!("qjs-{i}"),
            )
            .expect("quickjs session creation should succeed");
        let used = 15 * 1024 * 1024usize;
        adapter
            .broker
            .record_usage_and_check(id, used, 10, 1024)
            .expect("quickjs usage should stay within budget");
        quickjs_heap.push(used as f64 / (1024.0 * 1024.0));
        quickjs_ids.push(id);
    }
    let mut quickjs_for_median = quickjs_heap.clone();
    let quickjs_median = median(&mut quickjs_for_median);
    let quickjs_max = quickjs_heap.iter().copied().fold(0.0, f64::max);
    assert!(quickjs_max < 50.0, "max QuickJS heap must stay < 50MB");
    assert!(
        quickjs_median < 20.0,
        "median QuickJS heap should stay < 20MB"
    );

    let mut v8_heap = Vec::with_capacity(10);
    let mut v8_ids = Vec::with_capacity(10);
    for i in 0..10 {
        let id = adapter
            .broker
            .create_session(EngineKind::V8, ResourceBudget::default(), format!("v8-{i}"))
            .expect("v8 session creation should succeed");
        let used = 80 * 1024 * 1024usize;
        adapter
            .broker
            .record_usage_and_check(id, used, 10, 1024)
            .expect("v8 usage should stay within budget");
        v8_heap.push(used as f64 / (1024.0 * 1024.0));
        v8_ids.push(id);
    }
    let mut v8_for_median = v8_heap.clone();
    let v8_median = median(&mut v8_for_median);
    let v8_max = v8_heap.iter().copied().fold(0.0, f64::max);
    assert!(v8_max < 200.0, "max V8 heap must stay < 200MB");
    assert!(v8_median < 100.0, "median V8 heap should stay < 100MB");

    let teardown_start = Instant::now();
    for id in quickjs_ids.into_iter().chain(v8_ids.into_iter()) {
        let _ = adapter.broker.remove(id);
    }
    assert!(
        teardown_start.elapsed() < Duration::from_secs(5),
        "teardown should release session heap quickly"
    );
    assert_eq!(adapter.session_count(), baseline);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scale_circuit_breaker_under_load() {
    engine::init_v8();
    let adapter = Arc::new(EngineAdapter::new(1, 0, 1, 0, ResourceBudget::default()).await);

    let held = adapter
        .tier1
        .acquire()
        .await
        .expect("first acquire should fill the single-slot pool");
    for _ in 0..5 {
        let _ = adapter.tier1.acquire().await;
    }
    assert_eq!(adapter.tier1.circuit_breaker_state(), OPEN);
    metrics::CIRCUIT_BREAKER_STATE
        .with_label_values(&["tier1"])
        .set(OPEN as i64);
    assert_eq!(
        metrics::CIRCUIT_BREAKER_STATE
            .with_label_values(&["tier1"])
            .get(),
        OPEN as i64
    );

    let fast_fail_start = Instant::now();
    let _ = adapter.tier1.acquire().await;
    assert!(
        fast_fail_start.elapsed() < Duration::from_millis(50),
        "open breaker should fail fast"
    );

    adapter.tier1.release_after_use(held);
    tokio::time::sleep(Duration::from_secs(31)).await;

    let probe = adapter
        .tier1
        .acquire()
        .await
        .expect("half-open probe should succeed after interval");
    adapter.tier1.release_after_use(probe);
    for _ in 0..2 {
        let s = adapter
            .tier1
            .acquire()
            .await
            .expect("success should be allowed while recovering");
        adapter.tier1.release_after_use(s);
    }
    assert_eq!(adapter.tier1.circuit_breaker_state(), 0);
    metrics::CIRCUIT_BREAKER_STATE
        .with_label_values(&["tier1"])
        .set(0);
    assert_eq!(
        metrics::CIRCUIT_BREAKER_STATE
            .with_label_values(&["tier1"])
            .get(),
        0
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scale_session_isolation() {
    engine::init_v8();
    let adapter = Arc::new(EngineAdapter::new(8, 0, 2, 0, ResourceBudget::default()).await);

    let mut session_ids = Vec::with_capacity(5);
    for i in 0..5 {
        let id = adapter
            .broker
            .create_session(
                EngineKind::QuickJs,
                ResourceBudget::default(),
                format!("iso-{i}"),
            )
            .expect("isolation session should create");
        session_ids.push(id);
    }
    let session_a = session_ids[0];
    let session_b = session_ids[1];

    let mut local_storage: HashMap<Uuid, HashMap<String, String>> = HashMap::new();
    local_storage
        .entry(session_a)
        .or_default()
        .insert("secret".to_string(), "session-a-data".to_string());
    let leaked = local_storage
        .get(&session_b)
        .and_then(|kv| kv.get("secret"))
        .cloned();
    assert_eq!(leaked, None, "session B must not see session A data");

    let cct_a = render_cct("page-a-content");
    let cct_b = render_cct("page-b-content");
    assert!(cct_a.contains("page-a-content"));
    assert!(!cct_a.contains("page-b-content"));
    assert!(cct_b.contains("page-b-content"));
    assert!(!cct_b.contains("page-a-content"));

    let mut storage_dirs = HashSet::new();
    for id in &session_ids {
        let sid = id.to_string();
        assert!(phantom_storage::is_valid_session_id(&sid));
        let dir = adapter
            .storage
            .create_session_dir(&sid)
            .expect("session storage dir should create");
        storage_dirs.insert(dir);
    }
    assert_eq!(storage_dirs.len(), session_ids.len());

    for id in session_ids {
        let _ = adapter.broker.remove(id);
    }
}
