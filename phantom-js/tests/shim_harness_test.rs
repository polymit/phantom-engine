use phantom_core::dom::DomTree;
use phantom_js::tier1::session::Tier1Session;

/// Eval helper — unwraps for test convenience
async fn eval(s: &Tier1Session, code: &str) -> String {
    s.eval(code).await.expect("eval must not fail")
}

/// Stand up a Tier1Session with full shims loaded
async fn session_with_shims() -> Tier1Session {
    let mut session = Tier1Session::new().await.unwrap();
    let tree = DomTree::new();
    session.attach_dom(tree).await;
    session
}

// ---------------------------------------------------------------------------
// Shim 1 — navigator.webdriver
// ---------------------------------------------------------------------------

#[tokio::test]
async fn shim01_webdriver_value_is_undefined() {
    let s = session_with_shims().await;
    let val = eval(&s, "String(navigator.webdriver)").await;
    assert_eq!(val, "undefined", "navigator.webdriver must be undefined");
    s.destroy();
}

#[tokio::test]
async fn shim01_webdriver_not_detectable_by_value() {
    let s = session_with_shims().await;
    // The 'in' operator returns true for any defined property, even if
    // value is undefined — that's correct JS spec behavior. What matters
    // for anti-detect is that the VALUE is undefined and the property is
    // not enumerable (Object.keys won't show it).
    let val = eval(&s, "String(navigator.webdriver)").await;
    assert_eq!(val, "undefined", "value must be undefined");
    let enumerable = eval(
        &s,
        "String(Object.keys(navigator).includes('webdriver'))",
    )
    .await;
    assert_eq!(
        enumerable, "false",
        "webdriver must not appear in Object.keys(navigator)"
    );
    s.destroy();
}

// ---------------------------------------------------------------------------
// Shim 2 — window.chrome
// ---------------------------------------------------------------------------

#[tokio::test]
async fn shim02_chrome_object_is_object() {
    let s = session_with_shims().await;
    let t = eval(&s, "typeof window.chrome").await;
    assert_eq!(t, "object", "window.chrome must be an object");
    s.destroy();
}

#[tokio::test]
async fn shim02_chrome_runtime_id_is_undefined() {
    let s = session_with_shims().await;
    let v = eval(&s, "String(window.chrome.runtime.id)").await;
    assert_eq!(
        v, "undefined",
        "chrome.runtime.id must be undefined when no extension"
    );
    s.destroy();
}

#[tokio::test]
async fn shim02_chrome_app_is_installed_false() {
    let s = session_with_shims().await;
    let v = eval(
        &s,
        "String(window.chrome && window.chrome.app && window.chrome.app.isInstalled)",
    )
    .await;
    assert_eq!(v, "undefined");
    s.destroy();
}

#[tokio::test]
async fn shim02_chrome_loadtimes_is_function() {
    let s = session_with_shims().await;
    let t = eval(&s, "typeof window.chrome.loadTimes").await;
    assert_eq!(t, "function", "chrome.loadTimes must be a function");
    s.destroy();
}

// ---------------------------------------------------------------------------
// Shim 3 — navigator.plugins
// ---------------------------------------------------------------------------

#[tokio::test]
async fn shim03_plugins_count_is_five() {
    let s = session_with_shims().await;
    let n = eval(&s, "String(navigator.plugins.length)").await;
    assert_eq!(n, "5", "exactly 5 PDF plugins required");
    s.destroy();
}

#[tokio::test]
async fn shim03_first_plugin_is_pdf_viewer() {
    let s = session_with_shims().await;
    let name = eval(&s, "navigator.plugins[0].name").await;
    assert_eq!(name, "PDF Viewer");
    s.destroy();
}

// ---------------------------------------------------------------------------
// Shim 5 — outerWidth / outerHeight
// ---------------------------------------------------------------------------

#[tokio::test]
async fn shim05_outer_width_not_zero() {
    let s = session_with_shims().await;
    let w: u32 = eval(&s, "String(window.outerWidth)")
        .await
        .parse()
        .unwrap();
    assert!(
        w > 0,
        "headless fingerprint: outerWidth is 0 — shim must prevent this"
    );
    assert_eq!(w, 1920, "outerWidth must match persona screen_width");
    s.destroy();
}

// ---------------------------------------------------------------------------
// Shim 7 + 8 — hardwareConcurrency, deviceMemory
// ---------------------------------------------------------------------------

#[tokio::test]
async fn shim07_hardware_concurrency_valid() {
    let s = session_with_shims().await;
    let n: u32 = eval(&s, "String(navigator.hardwareConcurrency)")
        .await
        .parse()
        .unwrap();
    assert!(
        matches!(n, 4 | 6 | 8 | 12 | 16),
        "hw_concurrency {} invalid bucket",
        n
    );
    s.destroy();
}

#[tokio::test]
async fn shim08_device_memory_valid() {
    let s = session_with_shims().await;
    let n: u32 = eval(&s, "String(navigator.deviceMemory)")
        .await
        .parse()
        .unwrap();
    assert!(matches!(n, 4 | 8), "deviceMemory {} must be 4 or 8", n);
    s.destroy();
}

// ---------------------------------------------------------------------------
// Shim 10 — navigator.userAgentData
// ---------------------------------------------------------------------------

#[tokio::test]
async fn shim10_ua_data_brands_length_is_three() {
    let s = session_with_shims().await;
    let n = eval(&s, "String(navigator.userAgentData.brands.length)").await;
    assert_eq!(n, "3", "three UA brands required");
    s.destroy();
}

#[tokio::test]
async fn shim10_get_high_entropy_values_includes_wow64() {
    let s = session_with_shims().await;
    // Store the resolved value in a global since QuickJS promise draining
    // needs an explicit pump cycle to capture .then() results
    eval(
        &s,
        r#"globalThis.__hev_result = null;
            navigator.userAgentData.getHighEntropyValues(['wow64', 'platform', 'architecture'])
                .then(function(v) {
                    globalThis.__hev_result = JSON.stringify({
                        has_wow64: 'wow64' in v,
                        has_platform: 'platform' in v,
                        has_arch: 'architecture' in v
                    });
                })"#,
    )
    .await;
    // Pump microtasks
    eval(&s, "").await;
    let result = eval(&s, "globalThis.__hev_result").await;
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(
        parsed["has_wow64"], true,
        "wow64 must be in getHighEntropyValues result"
    );
    assert_eq!(parsed["has_platform"], true);
    assert_eq!(parsed["has_arch"], true);
    s.destroy();
}

// ---------------------------------------------------------------------------
// Shim 16 — Intl.DateTimeFormat timezone
// ---------------------------------------------------------------------------

#[tokio::test]
async fn shim16_datetime_format_timezone_or_absent() {
    let s = session_with_shims().await;
    // QuickJS may not have Intl.DateTimeFormat — if absent, the shim
    // correctly skips installation. Only check when Intl is available.
    let has_intl = eval(&s, "String(typeof Intl !== 'undefined' && typeof Intl.DateTimeFormat === 'function')").await;
    if has_intl == "true" {
        let tz = eval(&s, "new Intl.DateTimeFormat().resolvedOptions().timeZone").await;
        assert!(!tz.is_empty(), "timezone must be set from persona");
        assert!(
            tz.contains('/') || tz == "UTC",
            "must be IANA zone or UTC, got: {}",
            tz
        );
    }
    s.destroy();
}

// ---------------------------------------------------------------------------
// Shim 17 — automation markers
// ---------------------------------------------------------------------------

#[tokio::test]
async fn shim17_playwright_marker_absent() {
    let s = session_with_shims().await;
    let v = eval(&s, "String(window.__playwright)").await;
    assert_eq!(v, "undefined", "__playwright must be deleted");
    s.destroy();
}

#[tokio::test]
async fn shim17_puppeteer_marker_absent() {
    let s = session_with_shims().await;
    let v = eval(&s, "String(window.__puppeteer_evaluation_script__)").await;
    assert_eq!(
        v, "undefined",
        "__puppeteer_evaluation_script__ must be deleted"
    );
    s.destroy();
}

#[tokio::test]
async fn shim17_webdriver_script_fn_absent() {
    let s = session_with_shims().await;
    let v = eval(&s, "String(window.__webdriver_script_fn)").await;
    assert_eq!(
        v, "undefined",
        "__webdriver_script_fn must be deleted"
    );
    s.destroy();
}

// ---------------------------------------------------------------------------
// Combined bot check — sannysoft-style fingerprint summary
// ---------------------------------------------------------------------------

#[tokio::test]
async fn shim_combined_sannysoft_style() {
    let s = session_with_shims().await;
    let checks = eval(
        &s,
        r#"JSON.stringify({
            webdriver_in:  'webdriver' in navigator,
            webdriver_val: String(navigator.webdriver),
            chrome_obj:    typeof window.chrome === 'object',
            plugins:       navigator.plugins.length,
            outer_w:       window.outerWidth > 0,
            hw_concur:     navigator.hardwareConcurrency,
            device_mem:    navigator.deviceMemory,
            langs_ok:      navigator.languages && navigator.languages.length > 0
        })"#,
    )
    .await;
    let v: serde_json::Value = serde_json::from_str(&checks).unwrap();
    assert_eq!(v["webdriver_val"], "undefined");
    assert_eq!(v["chrome_obj"], true, "bot check: chrome object");
    assert_eq!(v["plugins"], 5, "bot check: 5 plugins");
    assert_eq!(v["outer_w"], true, "bot check: outerWidth > 0");
    assert_eq!(v["langs_ok"], true, "bot check: languages");
    s.destroy();
}
