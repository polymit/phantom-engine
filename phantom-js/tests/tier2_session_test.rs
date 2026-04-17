#![allow(clippy::unwrap_used, clippy::expect_used)]
#[test]
fn test_tier2_snapshot_loads() {
    // This verifies:
    // 1. build.rs ran successfully (snapshot exists)
    // 2. JsRuntime loads from snapshot without panic
    // 3. Basic JS evaluates correctly
    let mut session = phantom_js::tier2::session::Tier2Session::new(None)
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
    let mut session = phantom_js::tier2::session::Tier2Session::new(None).unwrap();

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

    let stable_rtt = session
        .eval(
            "(() => {
                const reads = [];
                for (let i = 0; i < 10; i++) reads.push(navigator.connection.rtt);
                return String(new Set(reads).size === 1);
            })()",
        )
        .unwrap();
    assert_eq!(
        stable_rtt, "true",
        "navigator.connection.rtt must be stable across reads"
    );

    session.destroy();
}

#[test]
fn test_tier2_datetimeformat_supported_locales_of_is_preserved() {
    let mut session = phantom_js::tier2::session::Tier2Session::new(None).unwrap();

    let has_method = session
        .eval("typeof Intl.DateTimeFormat.supportedLocalesOf")
        .unwrap();
    assert_eq!(
        has_method, "function",
        "Intl.DateTimeFormat.supportedLocalesOf must be preserved by shim"
    );

    let callable = session
        .eval("String(Array.isArray(Intl.DateTimeFormat.supportedLocalesOf(['en-US'])))")
        .unwrap();
    assert_eq!(callable, "true", "supportedLocalesOf must remain callable");

    session.destroy();
}

#[test]
fn test_tier2_html_element_inherits_event_target_when_present() {
    let mut session = phantom_js::tier2::session::Tier2Session::new(None).unwrap();
    let has_event_methods = session
        .eval(
            "String(typeof HTMLElement === 'undefined' || typeof HTMLElement.prototype.addEventListener === 'function')",
        )
        .unwrap();
    assert_eq!(
        has_event_methods, "true",
        "HTMLElement.prototype must inherit EventTarget methods in snapshot"
    );
    session.destroy();
}

#[test]
fn test_tier2_session_isolation() {
    // Two Tier2 sessions must not share globals — D-40
    let mut s1 = phantom_js::tier2::session::Tier2Session::new(None).unwrap();
    let mut s2 = phantom_js::tier2::session::Tier2Session::new(None).unwrap();

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
    let s = phantom_js::tier2::session::Tier2Session::new(None).unwrap();
    s.destroy();

    // Measure hot path
    let start = Instant::now();
    let s = phantom_js::tier2::session::Tier2Session::new(None).unwrap();
    let elapsed = start.elapsed();
    println!("Tier2Session startup (hot): {:?}", elapsed);
    // Target: <50ms (minimum from blueprint)
    assert!(
        elapsed.as_millis() < 5000,
        "Tier2 session startup must be under 5 seconds even in debug"
    );
    s.destroy();
}

#[test]
fn test_tier2_pool_hard_cap_under_contention() {
    use phantom_js::error::PhantomJsError;
    use phantom_js::tier2::pool::Tier2Pool;

    let pool = Tier2Pool::new(2, 0, None);
    let s1 = pool.acquire().expect("first acquire must succeed");
    let s2 = pool.acquire().expect("second acquire must succeed");
    let third = pool.acquire();

    match third {
        Err(PhantomJsError::PoolExhausted { max }) => assert_eq!(max, 2),
        Ok(_) => panic!("third acquire unexpectedly succeeded with max_count=2"),
        Err(err) => panic!("unexpected acquire error: {err:?}"),
    }

    s2.destroy();
    s1.destroy();
}

#[test]
fn test_tier2_set_persona_rejects_invalid_json() {
    use phantom_js::error::PhantomJsError;

    let mut session = phantom_js::tier2::session::Tier2Session::new(None).unwrap();
    let err = session
        .set_persona("{not_json")
        .expect_err("invalid persona JSON must fail");

    assert!(
        matches!(err, PhantomJsError::Internal(msg) if msg.contains("invalid persona JSON")),
        "set_persona must surface invalid JSON as internal validation error"
    );

    session.destroy();
}

#[test]
fn test_tier2_set_persona_does_not_execute_payload() {
    let mut session = phantom_js::tier2::session::Tier2Session::new(None).unwrap();

    session.eval("globalThis.__persona_pwned = false").unwrap();
    let payload = r#"{"language":"en-US\"; globalThis.__persona_pwned = true; //"}"#;
    session
        .set_persona(payload)
        .expect("valid JSON payload must apply safely");

    let pwned = session.eval("String(globalThis.__persona_pwned)").unwrap();
    assert_eq!(
        pwned, "false",
        "set_persona must treat payload as data, not executable JS"
    );

    session.destroy();
}
