#![allow(clippy::unwrap_used, clippy::expect_used)]
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use phantom_core::process_html;
use phantom_serializer::{
    DeltaEngine, HeadlessSerializer, RawMutation, SerialiserConfig, SerialiserMode,
};
use std::time::Instant;

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

fn make_page_html(node_count: usize) -> String {
    let mut html = String::from("<html><body style='width: 1280px; height: 720px;'>");
    for i in 0..node_count {
        html.push_str(&format!(
            "<div id='node-{i}' style='width: 50px; height: 20px;'><button aria-label='btn-{i}' style='width: 40px; height: 18px;'>Click</button></div>"
        ));
    }
    html.push_str("</body></html>");
    html
}

fn bench_cct_full_1000_nodes(c: &mut Criterion) {
    let html = make_page_html(1000);
    let page = process_html(&html, "https://bench.full", 1280.0, 720.0)
        .expect("benchmark page parse should succeed");
    let config = SerialiserConfig {
        url: "https://bench.full".to_string(),
        mode: SerialiserMode::Full,
        ..Default::default()
    };

    let start = Instant::now();
    let warm = HeadlessSerializer::serialise(&page, &config);
    let measured_ms = start.elapsed().as_secs_f64() * 1000.0;
    assert_goal("cct_full_1000_nodes", measured_ms, 5.0, 10.0);
    black_box(warm.len());

    c.bench_function("cct_full_1000_nodes", |b| {
        b.iter(|| {
            black_box(HeadlessSerializer::serialise(
                black_box(&page),
                black_box(&config),
            ))
        })
    });
}

fn bench_cct_selective_1000_nodes(c: &mut Criterion) {
    let html = make_page_html(1000);
    let page = process_html(&html, "https://bench.selective", 1280.0, 720.0)
        .expect("benchmark page parse should succeed");
    let config = SerialiserConfig {
        url: "https://bench.selective".to_string(),
        mode: SerialiserMode::Selective,
        task_hint: Some("find clickable button".to_string()),
        ..Default::default()
    };

    let start = Instant::now();
    let warm = HeadlessSerializer::serialise(&page, &config);
    let measured_ms = start.elapsed().as_secs_f64() * 1000.0;
    assert_goal("cct_selective_1000_nodes", measured_ms, 5.0, 10.0);
    let emitted_nodes = warm
        .lines()
        .filter(|line| !line.starts_with("##"))
        .count()
        .max(1);
    let tokens_per_node = warm.len() as f64 / emitted_nodes as f64;
    println!(
        "INFO: cct_selective_1000_nodes output_size={} bytes, tokens_per_node≈{:.2}",
        warm.len(),
        tokens_per_node
    );

    c.bench_function("cct_selective_1000_nodes", |b| {
        b.iter(|| {
            let out = HeadlessSerializer::serialise(black_box(&page), black_box(&config));
            black_box(out.len())
        })
    });
}

fn bench_cct_delta_10_mutations(c: &mut Criterion) {
    let html = make_page_html(32);
    let page = process_html(&html, "https://bench.delta", 1280.0, 720.0)
        .expect("benchmark page parse should succeed");
    let root = page
        .tree
        .document_root
        .expect("page should include a document root");
    let ids: Vec<_> = root.descendants(&page.tree.arena).take(16).collect();

    let mut warm_engine = DeltaEngine::with_window_ms(0);
    for i in 0..10 {
        warm_engine.push(RawMutation::AttrChanged {
            node_id: ids[i % ids.len()],
            attr: "class".to_string(),
            old_val: Some("old".to_string()),
            new_val: Some(format!("new-{i}")),
        });
    }
    let start = Instant::now();
    let warm = warm_engine.coalesce();
    let measured_ms = start.elapsed().as_secs_f64() * 1000.0;
    assert_goal("cct_delta_10_mutations", measured_ms, 1.0, 2.0);
    black_box(warm.len());

    c.bench_function("cct_delta_10_mutations", |b| {
        b.iter(|| {
            let mut engine = DeltaEngine::with_window_ms(0);
            for i in 0..10 {
                engine.push(RawMutation::AttrChanged {
                    node_id: ids[i % ids.len()],
                    attr: "class".to_string(),
                    old_val: Some("old".to_string()),
                    new_val: Some(format!("new-{i}")),
                });
            }
            black_box(engine.coalesce())
        })
    });
}

criterion_group!(
    benches,
    bench_cct_full_1000_nodes,
    bench_cct_selective_1000_nodes,
    bench_cct_delta_10_mutations
);
criterion_main!(benches);
