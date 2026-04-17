#![allow(clippy::unwrap_used, clippy::expect_used)]
#[test]
fn test_v8_platform_initialises_without_panic() {
    // This test verifies that init_v8_platform() does not crash.
    // It must be the ONLY test that calls this function — V8 can
    // only be initialised once per process. Multiple calls crash.
    //
    // If this test passes, V8 platform is correctly configured.
    // If this test panics or segfaults, the PKU crash is happening.
    //
    // NOTE: This test MUST be run alone using:
    // cargo test --package phantom-js v8_init -- --test-threads=1
    phantom_js::init_v8_platform();
    let version = deno_core::v8::V8::get_version();
    assert!(!version.is_empty(), "V8 version string must not be empty");
    println!("V8 version: {}", version);
    // Expected: something like "14.6.202.26"
    // If this prints a version — platform init is correct.
}
