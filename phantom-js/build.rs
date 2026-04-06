fn main() {
    // Tell cargo to re-run build.rs if JS files change
    println!("cargo:rerun-if-changed=js/browser_shims.js");
    println!("cargo:rerun-if-changed=js/event_target.js");
    println!("cargo:rerun-if-changed=js/mutation_observer.js");
    println!("cargo:rerun-if-changed=js/location.js");

    create_base_snapshot();
}

fn create_base_snapshot() {
    use deno_core::{JsRuntimeForSnapshot, RuntimeOptions};

    let mut runtime = JsRuntimeForSnapshot::new(RuntimeOptions {
        ..Default::default()
    });

    // Pre-inject a default persona for snapshot
    // Sessions will override this with their actual persona
    // when they load
    runtime.execute_script("<phantom_persona_default>", r#"
        globalThis.__phantom_persona = {
            screen_width: 1920,
            screen_height: 1080,
            hardware_concurrency: 8,
            device_memory: 8,
            language: 'en-US',
            languages: ['en-US', 'en'],
            timezone: 'America/New_York',
            canvas_noise_seed: 1n,
            webgl_vendor: 'Google Inc. (NVIDIA)',
            webgl_renderer: 'ANGLE (NVIDIA, NVIDIA GeForce RTX 3060 Ti Direct3D11 vs_5_0 ps_5_0, D3D11)',
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
        globalThis.document = {};
        globalThis.Plugin = class Plugin {};
        globalThis.PluginArray = class PluginArray {};
        globalThis.Notification = { permission: 'default' };
    "#).expect("default persona injection must not fail");

    // Pre-execute browser shims into the snapshot
    // This means every session loaded from this snapshot already has
    // all anti-detection shims applied — <0.1ms instead of ~1ms
    let shims = include_str!("js/browser_shims.js");
    runtime
        .execute_script("<phantom_shims>", shims)
        .expect("browser_shims.js must execute without error in snapshot");

    let event_target = include_str!("js/event_target.js");
    runtime
        .execute_script("<phantom_event_target>", event_target)
        .expect("event_target.js must execute without error");

    let mutation_observer = include_str!("js/mutation_observer.js");
    runtime
        .execute_script("<phantom_mutation_observer>", mutation_observer)
        .expect("mutation_observer.js must execute without error");

    let location = include_str!("js/location.js");
    runtime
        .execute_script("<phantom_location>", location)
        .expect("location.js must execute without error");

    // Create the snapshot blob
    let snapshot = runtime.snapshot();

    // Write to OUT_DIR where include_bytes! can find it
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR must be set by cargo");
    let snapshot_path = std::path::Path::new(&out_dir).join("PHANTOM_BASE_SNAPSHOT.bin");

    std::fs::write(&snapshot_path, &snapshot).expect("snapshot write must succeed");

    println!(
        "cargo:warning=PHANTOM_BASE_SNAPSHOT.bin created: {} bytes",
        snapshot.len()
    );
}
