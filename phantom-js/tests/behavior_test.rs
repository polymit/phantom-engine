#![allow(clippy::unwrap_used, clippy::expect_used)]
// behavior_test.rs — verifies statistical properties of BehaviorEngine
// and geometric properties of the Bezier mouse path generator.
//
// These values are calibrated to match real Chrome user timing from the
// DMTG paper (arXiv 2410.18233). Wrong parameters = ML-detectable.

#[test]
fn click_hesitation_has_correct_lognormal_distribution() {
    use phantom_js::BehaviorEngine;

    let engine = BehaviorEngine::new();

    // Sample 1000 values and verify statistical properties.
    // LogNormal(μ=4.2, σ=0.9) has median = e^4.2 ≈ 66.7ms.
    // Mean = e^(μ + σ²/2) = e^(4.2 + 0.405) ≈ 94ms (before clamp).
    let samples: Vec<u64> = (0..1000).map(|_| engine.click_hesitation_ms()).collect();

    let sum: u64 = samples.iter().sum();
    let mean = sum as f64 / samples.len() as f64;

    // With clamp(20, 500) the effective mean shifts slightly but stays in 50–200ms.
    // If this assertion fails the LogNormal parameters have been changed — revert them.
    println!("click_hesitation mean over 1000 samples: {:.1}ms", mean);
    assert!(
        mean > 50.0 && mean < 250.0,
        "click_hesitation mean must be 50–250ms, got {:.1}ms — \
         check LogNormal(4.2, 0.9) parameters in BehaviorEngine::new()",
        mean
    );

    // All values must respect the clamp bounds
    for &v in &samples {
        assert!(
            (20..=500).contains(&v),
            "click_hesitation value {} out of clamp range [20, 500]",
            v
        );
    }
}

#[test]
fn inter_action_delay_stays_within_bounds() {
    use phantom_js::BehaviorEngine;

    let engine = BehaviorEngine::new();
    for _ in 0..500 {
        let v = engine.inter_action_delay_ms();
        assert!(
            (50..=3000).contains(&v),
            "inter_action delay {} out of clamp range [50, 3000]",
            v
        );
    }
}

#[test]
fn char_typing_delay_stays_within_bounds() {
    use phantom_js::BehaviorEngine;

    let engine = BehaviorEngine::new();
    for _ in 0..500 {
        let v = engine.char_typing_delay_ms();
        assert!(
            (30..=500).contains(&v),
            "char_typing delay {} out of clamp range [30, 500]",
            v
        );
    }
}

#[test]
fn mouse_path_has_correct_shape() {
    use phantom_js::BehaviorEngine;

    let engine = BehaviorEngine::new();
    let from = (100.0_f64, 200.0_f64);
    let to = (500.0_f64, 400.0_f64);
    let path = engine.generate_mouse_path(from, to);

    // Blueprint: 20–40 sampled points → 21–41 elements (0..=n inclusive)
    assert!(
        path.len() >= 21 && path.len() <= 41,
        "mouse path must have 21–41 points, got {}",
        path.len()
    );

    // First point must land on `from`
    let (x0, y0) = path[0];
    assert!(
        (x0 - from.0).abs() < 1.0,
        "path[0].x = {:.2}, expected {:.2}",
        x0,
        from.0
    );
    assert!(
        (y0 - from.1).abs() < 1.0,
        "path[0].y = {:.2}, expected {:.2}",
        y0,
        from.1
    );

    // Last point must land on `to`
    let (xn, yn) = *path.last().unwrap();
    assert!(
        (xn - to.0).abs() < 1.0,
        "path[last].x = {:.2}, expected {:.2}",
        xn,
        to.0
    );
    assert!(
        (yn - to.1).abs() < 1.0,
        "path[last].y = {:.2}, expected {:.2}",
        yn,
        to.1
    );

    // Consecutive points must not teleport — max 200px gap per step
    for i in 1..path.len() {
        let dx = path[i].0 - path[i - 1].0;
        let dy = path[i].1 - path[i - 1].1;
        let dist = (dx * dx + dy * dy).sqrt();
        assert!(
            dist < 200.0,
            "path step {} jumped {:.1}px — consecutive points must be continuous",
            i,
            dist
        );
    }
}

#[tokio::test]
async fn tier1_pool_acquire_and_release() {
    use phantom_js::tier1::pool::Tier1Pool;

    // Small pool so the test stays fast
    let pool = Tier1Pool::new(5, 2).await;

    let session = pool
        .acquire()
        .await
        .expect("pool must provide a session after pre-warm");

    let result = session.eval("'pool works'").await.unwrap();
    assert_eq!(result, "pool works");

    // D-40: release destroys the used session immediately.
    pool.release_after_use(session);

    // Pool must still be able to hand out a session
    let session2 = pool
        .acquire()
        .await
        .expect("pool must still work after release_after_use");

    let result2 = session2.eval("'pool still works'").await.unwrap();
    assert_eq!(result2, "pool still works");

    pool.release_after_use(session2);
}

#[tokio::test]
async fn tier1_pool_hard_cap_under_concurrent_acquire() {
    use phantom_js::error::PhantomJsError;
    use phantom_js::tier1::pool::Tier1Pool;

    let pool = Tier1Pool::new(2, 0).await;
    let (r1, r2, r3, r4) = tokio::join!(
        pool.acquire(),
        pool.acquire(),
        pool.acquire(),
        pool.acquire()
    );
    let mut sessions = Vec::new();
    for res in [r1, r2, r3, r4] {
        match res {
            Ok(session) => sessions.push(session),
            Err(PhantomJsError::PoolExhausted { .. }) => {}
            Err(err) => panic!("unexpected acquire error: {err:?}"),
        }
    }

    assert!(
        sessions.len() <= 2,
        "pool handed out {} sessions with max_count=2",
        sessions.len()
    );

    for session in sessions {
        pool.release_after_use(session);
    }
}
