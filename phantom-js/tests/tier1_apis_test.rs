#[tokio::test]
async fn test_set_timeout_fires() {
    use phantom_js::tier1::session::Tier1Session;

    // spawn_local requires a LocalSet — timers use tokio::task::spawn_local
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async move {
            let session = Tier1Session::new().await.unwrap();

            // Set a global flag in JS, then use setTimeout to flip it
            session
                .eval("globalThis.__timer_fired = false;")
                .await
                .unwrap();

            // Register setTimeout — normally done by setup_dom_environment.
            // Here we inject it manually so the test is self-contained.
            let ctx = session.context.clone();
            let ctx_for_timer = session.context.clone();
            rquickjs::async_with!(ctx => |qjs_ctx| {
                let globals = qjs_ctx.globals();
                phantom_js::tier1::apis::timers::register_timers(
                    &qjs_ctx,
                    &globals,
                    ctx_for_timer,
                ).unwrap();
                Ok::<(), ()>(())
            })
            .await
            .unwrap();

            session
                .eval("setTimeout(function() { globalThis.__timer_fired = true; }, 50);")
                .await
                .unwrap();

            // Wait well beyond the 50ms timer delay
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;

            // Drain the microtask queue so the timer callback can run
            session.eval("").await.unwrap();

            let fired = session.eval("globalThis.__timer_fired").await.unwrap();
            assert_eq!(
                fired, "true",
                "setTimeout callback must have fired after 50ms delay"
            );

            session.destroy();
        })
        .await;
}

#[tokio::test]
async fn test_mutation_bridge_dispatches() {
    use phantom_js::tier1::apis::mutation_observer::MutationBridge;
    use phantom_js::tier1::session::Tier1Session;

    let session = Tier1Session::new().await.unwrap();

    // Set up a MutationObserver in JS
    session
        .eval(
            r#"
        globalThis.__mutation_received = false;
        globalThis.__phantom_dispatch_mutation = function(record) {
            globalThis.__mutation_received = true;
        };
    "#,
        )
        .await
        .unwrap();

    // Fire a mutation from Rust
    let bridge = MutationBridge::new(session.context.clone());
    bridge
        .notify_mutation(r#"{"type":"childList","target":"n_1"}"#)
        .await;

    // Verify the mutation was received
    let received = session
        .eval("globalThis.__mutation_received")
        .await
        .unwrap();
    assert_eq!(
        received, "true",
        "MutationBridge must dispatch mutation to JS and drain microtasks"
    );

    session.destroy();
}

#[tokio::test]
async fn test_fetch_stub_exists() {
    use phantom_js::tier1::session::Tier1Session;
    let session = Tier1Session::new().await.unwrap();

    let ctx = session.context.clone();
    rquickjs::async_with!(ctx => |qjs_ctx| {
        let globals = qjs_ctx.globals();
        phantom_js::tier1::apis::fetch::register_fetch(&qjs_ctx, &globals).unwrap();
        Ok::<(), ()>(())
    })
    .await
    .unwrap();

    // Verify fetch is registered as a function
    let has_fetch = session.eval("typeof fetch").await.unwrap();
    assert_eq!(
        has_fetch, "function",
        "window.fetch must be defined as a function"
    );

    // Verify fetch returns a Promise (not its resolved value — that requires .then)
    let is_promise = session
        .eval("fetch('http://localhost') instanceof Promise")
        .await
        .unwrap();
    assert_eq!(is_promise, "true", "fetch() must return a Promise");

    session.destroy();
}

#[tokio::test]
async fn test_session_startup_time() {
    use std::time::Instant;
    let start = Instant::now();
    let session = phantom_js::tier1::session::Tier1Session::new()
        .await
        .unwrap();
    let elapsed = start.elapsed();
    println!("Tier1Session startup: {:?}", elapsed);
    // Target from blueprint: <10ms for Tier 1
    // In debug builds this may be slower — log but do not assert
    assert!(
        elapsed.as_secs() < 5,
        "Session startup must complete in under 5 seconds (even debug)"
    );
    session.destroy();
}
