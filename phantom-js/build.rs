fn main() {
    // Tell cargo to re-run build.rs if JS files change
    println!("cargo:rerun-if-changed=js/browser_shims.js");
    println!("cargo:rerun-if-changed=js/event_target.js");
    println!("cargo:rerun-if-changed=js/mutation_observer.js");
    println!("cargo:rerun-if-changed=js/location.js");

    create_snapshots();
}

#[derive(Clone, Copy)]
enum SnapshotFlavor {
    Base,
    React,
    Vue,
}

impl SnapshotFlavor {
    fn file_name(self) -> &'static str {
        match self {
            Self::Base => "PHANTOM_BASE_SNAPSHOT.bin",
            Self::React => "PHANTOM_REACT_SNAPSHOT.bin",
            Self::Vue => "PHANTOM_VUE_SNAPSHOT.bin",
        }
    }

    fn bootstrap_script(self) -> &'static str {
        match self {
            Self::Base => "globalThis.__phantom_snapshot = 'base';",
            Self::React => {
                r#"
                globalThis.__phantom_snapshot = 'react';
                globalThis.React = globalThis.React || {
                    version: '18.2.0',
                    createElement: function () {},
                };
                "#
            }
            Self::Vue => {
                r#"
                globalThis.__phantom_snapshot = 'vue';
                globalThis.Vue = globalThis.Vue || {
                    version: '3.4.0',
                    createApp: function () {
                        return { mount: function () {} };
                    },
                };
                "#
            }
        }
    }
}

fn create_snapshots() {
    create_snapshot(SnapshotFlavor::Base);
    create_snapshot(SnapshotFlavor::React);
    create_snapshot(SnapshotFlavor::Vue);
}

fn create_snapshot(flavor: SnapshotFlavor) {
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

    let event_target = include_str!("js/event_target.js");
    runtime
        .execute_script("<phantom_event_target>", event_target)
        .expect("event_target.js must execute without error");

    // Pre-execute browser shims into the snapshot.
    // event_target.js must be loaded first because shim #19 patches
    // HTMLElement.prototype to inherit from EventTarget.prototype.
    let shims = include_str!("js/browser_shims.js");
    runtime
        .execute_script("<phantom_shims>", shims)
        .expect("browser_shims.js must execute without error in snapshot");

    let mutation_observer = include_str!("js/mutation_observer.js");
    runtime
        .execute_script("<phantom_mutation_observer>", mutation_observer)
        .expect("mutation_observer.js must execute without error");

    let location = include_str!("js/location.js");
    runtime
        .execute_script("<phantom_location>", location)
        .expect("location.js must execute without error");

    runtime
        .execute_script("<phantom_snapshot_flavor>", flavor.bootstrap_script())
        .expect("snapshot bootstrap must execute without error");

    // Create the snapshot blob
    let snapshot = runtime.snapshot();

    // Write to OUT_DIR where include_bytes! can find it
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR must be set by cargo");
    let file_name = flavor.file_name();
    let snapshot_path = std::path::Path::new(&out_dir).join(file_name);

    std::fs::write(&snapshot_path, &snapshot).expect("snapshot write must succeed");

    println!(
        "cargo:warning={file_name} created: {} bytes",
        snapshot.len()
    );
}
