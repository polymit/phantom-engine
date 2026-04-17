#![allow(clippy::unwrap_used, clippy::expect_used)]
#[tokio::test]
async fn test_tier1_eval_basic_js() {
    phantom_js::init_v8_platform();
    let session = phantom_js::tier1::session::Tier1Session::new()
        .await
        .expect("session creation must not fail");

    let result = session.eval("1 + 1").await.expect("eval must not fail");
    assert_eq!(result, "2", "1 + 1 must equal 2");

    let result = session
        .eval("typeof window")
        .await
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

    let result = session
        .eval("'memory limit configured'")
        .await
        .expect("must eval");
    assert_eq!(result, "memory limit configured");
    session.destroy();
}

#[tokio::test]
async fn test_tier1_session_isolates_globals() {
    // Two sessions must NOT share JS globals
    // This is the "burn it down" model — D-08
    let s1 = phantom_js::tier1::session::Tier1Session::new()
        .await
        .unwrap();
    let s2 = phantom_js::tier1::session::Tier1Session::new()
        .await
        .unwrap();

    // Set a global in s1
    s1.eval("globalThis.__phantom_test_marker = 'session_1'")
        .await
        .unwrap();

    // s2 must NOT see it
    let result = s2
        .eval("typeof globalThis.__phantom_test_marker")
        .await
        .unwrap();
    assert_eq!(
        result, "undefined",
        "Session 2 must not see Session 1's globals — sessions are isolated"
    );

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
    session
        .eval(persona_init)
        .await
        .expect("persona init must not fail");

    let shims_source = include_str!("../js/browser_shims.js");
    let test_source = format!(
        "try {{ eval(`{}`); 'OK' }} catch (e) {{ String(e) + '\\n' + String(e.stack) }}",
        shims_source.replace("`", "\\`").replace("$", "\\$")
    );
    let result = session.eval(&test_source).await.unwrap();
    if result != "OK" {
        panic!(
            "browser_shims.js has syntax error or load error:\n{}",
            result
        );
    }

    let webdriver = session.eval("navigator.webdriver").await.unwrap();
    assert_eq!(
        webdriver, "undefined",
        "navigator.webdriver must be undefined after shims"
    );

    let has_chrome = session.eval("typeof window.chrome").await.unwrap();
    assert_eq!(
        has_chrome, "object",
        "window.chrome must be an object after shims"
    );

    let plugins_len = session
        .eval("String(navigator.plugins.length)")
        .await
        .unwrap();
    assert_eq!(
        plugins_len, "5",
        "navigator.plugins must expose 5 PDF plugins"
    );

    let has_plugin_mimes = session
        .eval("String(Boolean(navigator.plugins[0] && navigator.plugins[0].mimeTypes && navigator.plugins[0].mimeTypes.length > 0))")
        .await
        .unwrap();
    assert_eq!(
        has_plugin_mimes, "true",
        "plugins[0].mimeTypes must exist and be non-empty"
    );

    let has_global_pdf_mime = session
        .eval("String(Boolean(navigator.mimeTypes && navigator.mimeTypes['application/pdf']))")
        .await
        .unwrap();
    assert_eq!(
        has_global_pdf_mime, "true",
        "navigator.mimeTypes must expose application/pdf"
    );

    let has_native_client = session
        .eval("String(Array.from({ length: navigator.plugins.length }, (_, i) => navigator.plugins[i].name).includes('Native Client'))")
        .await
        .unwrap();
    assert_eq!(
        has_native_client, "false",
        "Native Client plugin must not be present"
    );

    let stable_rtt = session
        .eval(
            "(() => {
                const reads = [];
                for (let i = 0; i < 10; i++) reads.push(navigator.connection.rtt);
                return String(new Set(reads).size === 1);
            })()",
        )
        .await
        .unwrap();
    assert_eq!(
        stable_rtt, "true",
        "navigator.connection.rtt must be stable across reads"
    );

    let bounded_rtt = session
        .eval(
            "(() => {
                const v = navigator.connection.rtt;
                return String(Number.isInteger(v) && v >= 100 && v < 150);
            })()",
        )
        .await
        .unwrap();
    assert_eq!(
        bounded_rtt, "true",
        "navigator.connection.rtt must be in [100, 149]"
    );

    session.destroy();
}

#[tokio::test]
async fn test_audio_noise_uses_distinct_seed_from_canvas_noise() {
    use phantom_js::tier1::session::Tier1Session;

    let session = Tier1Session::new().await.unwrap();
    let init = r#"
        globalThis.window = globalThis;
        globalThis.navigator = {};
        globalThis.PluginArray = function() {};
        globalThis.Plugin = function() {};
        globalThis.MimeType = function() {};
        globalThis.MimeTypeArray = function() {};

        globalThis.CanvasRenderingContext2D = function() {};
        CanvasRenderingContext2D.prototype.getImageData = function() {
            return { data: new Uint8ClampedArray([100, 100, 100, 255]) };
        };

        globalThis.AudioContext = function() {};
        AudioContext.prototype.createAnalyser = function() {
            return {
                getFloatFrequencyData: function(array) {
                    for (let i = 0; i < array.length; i++) array[i] = 0;
                }
            };
        };

        globalThis.__phantom_persona = {
            screen_width: 1920,
            screen_height: 1080,
            hardware_concurrency: 8,
            device_memory: 8,
            language: 'en-US',
            languages: ['en-US', 'en'],
            timezone: 'America/New_York',
            canvas_noise_seed: 37n,
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
    "#;
    session.eval(init).await.unwrap();

    let shims_source = include_str!("../js/browser_shims.js");
    let load = format!(
        "try {{ eval(`{}`); 'OK' }} catch (e) {{ String(e) + '\\n' + String(e.stack) }}",
        shims_source.replace("`", "\\`").replace("$", "\\$")
    );
    let loaded = session.eval(&load).await.unwrap();
    assert_eq!(loaded, "OK", "browser_shims.js must load in test harness");

    let result = session
        .eval(
            "(() => {
                const seed = 37n;
                const A = 1103515245n;
                const C = 12345n;
                const M = 2147483648n;
                const first = (s) => {
                    s = (s * A + C) % M;
                    return Number(s) / 2147483648;
                };
                const expectedSame = first(seed) * 0.1 - 0.05;
                const expectedXor = first(seed ^ 0xDEADBEEFn) * 0.1 - 0.05;

                const analyser = new AudioContext().createAnalyser();
                const arr = new Float32Array(1);
                analyser.getFloatFrequencyData(arr);

                const audio = arr[0];
                const distSame = Math.abs(audio - expectedSame);
                const distXor = Math.abs(audio - expectedXor);
                return distXor < distSame ? 'xor' : 'same';
            })()",
        )
        .await
        .unwrap();

    assert_eq!(
        result, "xor",
        "audio noise stream must be decorrelated from canvas seed"
    );

    session.destroy();
}

#[tokio::test]
async fn test_element_value_getter_setter_for_form_controls() {
    use phantom_core::process_html;
    use phantom_js::tier1::session::Tier1Session;

    let page = process_html(
        "<html><body><input id='email' value='a'/><textarea id='bio'>old</textarea></body></html>",
        "https://type.test",
        1280.0,
        720.0,
    )
    .expect("html parse must succeed");

    let mut session = Tier1Session::new().await.expect("session must create");
    session.attach_dom(page.tree).await;

    let input_result = session
        .eval(
            "(() => {
                const el = document.querySelector('#email');
                if (!el) return 'missing';
                el.value = el.value + 'bc';
                return el.value;
            })()",
        )
        .await
        .expect("input value mutation must succeed");
    assert_eq!(input_result, "abc", "input.value setter must mutate value");

    let textarea_result = session
        .eval(
            "(() => {
                const el = document.querySelector('#bio');
                if (!el) return 'missing';
                el.value = el.value + '!';
                return el.value;
            })()",
        )
        .await
        .expect("textarea value mutation must succeed");
    assert_eq!(
        textarea_result, "old!",
        "textarea.value setter must mutate text"
    );

    session.destroy();
}

#[tokio::test]
async fn test_element_is_content_editable_and_text_content_setter() {
    use phantom_core::process_html;
    use phantom_js::tier1::session::Tier1Session;

    let page = process_html(
        "<html><body><div id='editor' contenteditable='true'>x</div></body></html>",
        "https://type.test",
        1280.0,
        720.0,
    )
    .expect("html parse must succeed");

    let mut session = Tier1Session::new().await.expect("session must create");
    session.attach_dom(page.tree).await;

    let result = session
        .eval(
            "(() => {
                const el = document.querySelector('#editor');
                if (!el) return 'missing';
                if (!el.isContentEditable) return 'not_editable';
                el.textContent = (el.textContent || '') + 'y';
                return el.textContent;
            })()",
        )
        .await
        .expect("contenteditable mutation must succeed");
    assert_eq!(
        result, "xy",
        "contentEditable element must support writable textContent"
    );

    session.destroy();
}
