#[test]
fn test_tier2_snapshot_loads() {
    // This verifies:
    // 1. build.rs ran successfully (snapshot exists)
    // 2. JsRuntime loads from snapshot without panic
    // 3. Basic JS evaluates correctly
    let mut session = phantom_js::tier2::session::Tier2Session::new()
        .expect("Tier2Session::new() must not fail — snapshot must exist");

    let result = session.eval("1 + 1").expect("eval must not fail");
    assert_eq!(
        result, "2",
        "Basic arithmetic must work after snapshot load"
    );

    session.destroy();
}

#[test]
fn test_tier2_shims_in_snapshot() {
    // Verify browser shims are pre-applied in the snapshot
    let mut session = phantom_js::tier2::session::Tier2Session::new().unwrap();

    // navigator.webdriver must be undefined (shim applied)
    let webdriver = session.eval("String(navigator.webdriver)").unwrap();
    assert_eq!(
        webdriver, "undefined",
        "navigator.webdriver shim must be applied from snapshot"
    );

    // window.chrome must exist (shim applied)
    let chrome = session.eval("typeof window.chrome").unwrap();
    assert_eq!(
        chrome, "object",
        "window.chrome shim must be applied from snapshot"
    );

    session.destroy();
}

#[test]
fn test_tier2_session_isolation() {
    // Two Tier2 sessions must not share globals — D-40
    let mut s1 = phantom_js::tier2::session::Tier2Session::new().unwrap();
    let mut s2 = phantom_js::tier2::session::Tier2Session::new().unwrap();

    s1.eval("globalThis.__tier2_marker = 'session_one'")
        .unwrap();
    let s2_result = s2.eval("typeof globalThis.__tier2_marker").unwrap();

    assert_eq!(
        s2_result, "undefined",
        "Tier2 sessions must be fully isolated — globals must not leak"
    );

    s2.destroy();
    s1.destroy();
}

#[test]
fn test_tier2_startup_time() {
    use std::time::Instant;
    // Warm up — first load is slower due to OS file caching
    let s = phantom_js::tier2::session::Tier2Session::new().unwrap();
    s.destroy();

    // Measure hot path
    let start = Instant::now();
    let s = phantom_js::tier2::session::Tier2Session::new().unwrap();
    let elapsed = start.elapsed();
    println!("Tier2Session startup (hot): {:?}", elapsed);
    // Target: <50ms (minimum from blueprint)
    assert!(
        elapsed.as_millis() < 5000,
        "Tier2 session startup must be under 5 seconds even in debug"
    );
    s.destroy();
}
