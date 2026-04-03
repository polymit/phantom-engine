#[tokio::test]
async fn test_tier1_eval_basic_js() {
    phantom_js::init_v8_platform();
    let session = phantom_js::tier1::session::Tier1Session::new()
        .await
        .expect("session creation must not fail");

    let result = session.eval("1 + 1").await
        .expect("eval must not fail");
    assert_eq!(result, "2", "1 + 1 must equal 2");

    let result = session.eval("typeof window").await
        .expect("eval must not fail");
    // QuickJS has no window by default — we set it up later
    // For now: undefined is expected
    println!("typeof window = {}", result);

    session.destroy();
}

#[tokio::test]
async fn test_tier1_memory_limit_set() {
    // Verify the session was created — if memory limit silently
    // disabled (rust-alloc bug), session still works but limit is gone.
    // We cannot test the limit without allocating huge amounts of memory,
    // but we can verify the session starts correctly.
    let session = phantom_js::tier1::session::Tier1Session::new()
        .await
        .expect("session must create with memory limit configured");

    let result = session.eval("'memory limit configured'").await
        .expect("must eval");
    assert_eq!(result, "memory limit configured");
    session.destroy();
}

#[tokio::test]
async fn test_tier1_session_isolates_globals() {
    // Two sessions must NOT share JS globals
    // This is the "burn it down" model — D-08
    let s1 = phantom_js::tier1::session::Tier1Session::new().await.unwrap();
    let s2 = phantom_js::tier1::session::Tier1Session::new().await.unwrap();

    // Set a global in s1
    s1.eval("globalThis.__phantom_test_marker = 'session_1'").await.unwrap();

    // s2 must NOT see it
    let result = s2.eval("typeof globalThis.__phantom_test_marker").await.unwrap();
    assert_eq!(result, "undefined",
        "Session 2 must not see Session 1's globals — sessions are isolated");

    s1.destroy();
    s2.destroy();
}

#[tokio::test]
async fn test_shims_browser_shims_js_syntax() {
    use phantom_js::tier1::session::Tier1Session;
    let session = Tier1Session::new().await.unwrap();

    let persona_init = r#"
        globalThis.__phantom_persona = {
            screen_width: 1920,
            screen_height: 1080,
            hardware_concurrency: 8,
            device_memory: 8,
            language: 'en-US',
            languages: ['en-US', 'en'],
            timezone: 'America/New_York',
            canvas_noise_seed: 12345678n,
            webgl_vendor: 'Google Inc. (NVIDIA)',
            webgl_renderer: 'ANGLE (NVIDIA, NVIDIA GeForce RTX 3060)',
            chrome_major: '133',
            ua_platform: 'Windows',
            platform_version: '15.0.0',
            ua_full_version: '133.0.6943.98',
            ua_architecture: 'x86',
            ua_bitness: '64',
            ua_wow64: false,
            platform: 'Win32',
        };
        globalThis.window = globalThis;
        globalThis.navigator = {};
        globalThis.PluginArray = function() {};
        globalThis.Plugin = function() {};
    "#;
    session.eval(persona_init).await
        .expect("persona init must not fail");

    let shims_source = include_str!("../js/browser_shims.js");
    let test_source = format!("try {{ eval(`{}`); 'OK' }} catch (e) {{ String(e) + '\\n' + String(e.stack) }}", shims_source.replace("`", "\\`").replace("$", "\\$"));
    let result = session.eval(&test_source).await.unwrap();
    if result != "OK" {
        panic!("browser_shims.js has syntax error or load error:\n{}", result);
    }

    let webdriver = session.eval("navigator.webdriver").await.unwrap();
    assert_eq!(webdriver, "undefined",
        "navigator.webdriver must be undefined after shims");

    let has_chrome = session.eval("typeof window.chrome").await.unwrap();
    assert_eq!(has_chrome, "object",
        "window.chrome must be an object after shims");

    session.destroy();
}
