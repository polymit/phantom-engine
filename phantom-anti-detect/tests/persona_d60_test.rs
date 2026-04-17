#![allow(clippy::unwrap_used, clippy::expect_used)]
use phantom_anti_detect::{ChromeProfile, GpuProfile, Persona, PersonaPool};
use std::collections::HashSet;

#[test]
fn d60_all_fields_present() {
    let persona = Persona::win11_chrome133_nvidia_rtx3060ti(42);
    assert!(
        !persona.platform_version.is_empty(),
        "platform_version missing"
    );
    assert!(
        !persona.ua_full_version.is_empty(),
        "ua_full_version missing"
    );
    assert!(
        !persona.ua_architecture.is_empty(),
        "ua_architecture missing"
    );
    assert!(!persona.ua_bitness.is_empty(), "ua_bitness missing");
    assert!(!persona.ua_platform.is_empty(), "ua_platform missing");
    assert!(!persona.chrome_major.is_empty(), "chrome_major missing");
    assert!(persona.screen_width > 0, "screen_width missing");
    assert!(persona.screen_height > 0, "screen_height missing");
    assert!(
        persona.device_pixel_ratio > 0.0,
        "device_pixel_ratio missing"
    );
}

#[test]
fn hardware_concurrency_never_one_two_or_extreme() {
    let pool = PersonaPool::default_pool();
    for i in 0..pool.len() {
        let p = pool
            .clone_persona(i)
            .expect("persona index within pool bounds");
        assert!(
            matches!(p.hardware_concurrency, 4 | 6 | 8 | 12 | 16),
            "hardware_concurrency {} invalid — D-60 bans 1, 2, 128",
            p.hardware_concurrency
        );
    }
}

#[test]
fn device_memory_is_bucketed() {
    let pool = PersonaPool::default_pool();
    for i in 0..pool.len() {
        let p = pool
            .clone_persona(i)
            .expect("persona index within pool bounds");
        assert!(
            matches!(p.device_memory, 4 | 8),
            "device_memory {} must be 4 or 8 GB bucket — D-60",
            p.device_memory
        );
    }
}

#[test]
fn chrome133_platform_version_windows11() {
    let p = Persona::win11_chrome133_nvidia_rtx3060ti(1);
    assert_eq!(p.platform_version, "15.0.0", "Win11 must be 15.0.0");
    assert_eq!(p.ua_platform, "Windows");
    assert_eq!(p.platform, "Win32");
}

#[test]
fn chrome134_platform_version_windows11() {
    let p = Persona::win11_chrome134_nvidia_rtx3060ti(1);
    assert_eq!(p.platform_version, "15.0.0");
    assert_eq!(
        p.ua_full_version, "134.0.6998.36",
        "exact blueprint version"
    );
    assert_eq!(p.chrome_major, "134");
}

#[test]
fn win10_platform_version_is_10() {
    let p = Persona::win10_chrome133_intel_uhd770(1);
    assert_eq!(p.platform_version, "10.0.0", "Win10 must be 10.0.0");
}

#[test]
fn macos_platform_version_is_14() {
    let p = Persona::macos_sonoma_chrome133_m3pro(1);
    assert_eq!(p.platform_version, "14.0.0", "macOS Sonoma must be 14.0.0");
    assert_eq!(p.ua_architecture, "arm", "M3 is ARM not x86 — D-56");
    assert_eq!(p.ua_platform, "macOS");
    assert_eq!(p.platform, "MacIntel");
}

#[test]
fn chrome_major_matches_user_agent() {
    let p133 = Persona::win11_chrome133_nvidia_rtx3060ti(1);
    assert_eq!(p133.chrome_major, "133");
    assert!(
        p133.user_agent.contains("Chrome/133"),
        "user agent missing Chrome/133"
    );
    assert!(
        p133.ua_full_version.starts_with("133."),
        "ua full version incorrect"
    );

    let p134 = Persona::win11_chrome134_nvidia_rtx3060ti(1);
    assert_eq!(p134.chrome_major, "134");
    assert!(
        p134.user_agent.contains("Chrome/134"),
        "user agent missing Chrome/134"
    );
}

#[test]
fn gpu_profiles_exact_strings_nvidia() {
    let (v, r) = GpuProfile::WindowsNvidiaRtx3060Ti.strings();
    assert_eq!(v, "Google Inc. (NVIDIA)");
    assert_eq!(
        r,
        "ANGLE (NVIDIA, NVIDIA GeForce RTX 3060 Ti Direct3D11 vs_5_0 ps_5_0, D3D11)"
    );
}

#[test]
fn gpu_profiles_exact_strings_amd() {
    let (v, r) = GpuProfile::WindowsAmdRx6600.strings();
    assert_eq!(v, "Google Inc. (AMD)");
    assert_eq!(
        r,
        "ANGLE (AMD, AMD Radeon RX 6600 Direct3D11 vs_5_0 ps_5_0, D3D11)"
    );
}

#[test]
fn gpu_profiles_exact_strings_intel() {
    let (v, r) = GpuProfile::WindowsIntelUhd770.strings();
    assert_eq!(v, "Google Inc. (Intel)");
    assert_eq!(
        r,
        "ANGLE (Intel, Intel(R) UHD Graphics 770 Direct3D11 vs_5_0 ps_5_0, D3D11)"
    );
}

#[test]
fn gpu_profiles_exact_strings_apple_m3() {
    let (v, r) = GpuProfile::MacOsAppleM3Pro.strings();
    assert_eq!(v, "Google Inc.");
    assert_eq!(
        r,
        "ANGLE (Apple, ANGLE Metal Renderer: Apple M3 Pro, Unspecified Version)"
    );
}

#[test]
fn no_swiftshader_or_mesa_in_any_profile() {
    let profiles = [
        GpuProfile::WindowsNvidiaRtx3060Ti,
        GpuProfile::WindowsAmdRx6600,
        GpuProfile::WindowsIntelUhd770,
        GpuProfile::MacOsAppleM3Pro,
        GpuProfile::MacOsIntelIrisPro,
        GpuProfile::WindowsNvidiaRtx4070,
    ];
    for profile in profiles {
        let (vendor, renderer) = profile.strings();
        assert!(
            !vendor.contains("SwiftShader"),
            "SwiftShader = instant bot flag D-53"
        );
        assert!(
            !renderer.contains("SwiftShader"),
            "SwiftShader = instant bot flag D-53"
        );
        assert!(
            !vendor.contains("Mesa"),
            "Mesa in WebGL is generic and flags bots"
        );
        assert!(
            !renderer.contains("Mesa"),
            "Mesa in WebGL is generic and flags bots"
        );
        assert!(
            !renderer.contains("llvmpipe"),
            "llvmpipe is software rendering and flags bots"
        );
        assert!(
            vendor.starts_with("Google Inc"),
            "vendor missing Google Inc prefix"
        );
    }
}

#[test]
fn pool_has_five_personas() {
    let pool = PersonaPool::default_pool();
    assert_eq!(pool.len(), 5, "blueprint Section 6.8.2 — 5 combinations");
}

#[test]
fn pool_has_both_chrome_versions() {
    let pool = PersonaPool::default_pool();
    let mut count133 = 0;
    let mut count134 = 0;

    for i in 0..pool.len() {
        let p = pool
            .clone_persona(i)
            .expect("persona index within pool bounds");
        match p.chrome_version {
            ChromeProfile::Chrome133 => count133 += 1,
            ChromeProfile::Chrome134 => count134 += 1,
        }
    }

    assert!(count133 >= 3, "Chrome133 = 60% weight D-21");
    assert!(count134 >= 1, "Chrome134 = 40% weight D-21");
}

#[test]
fn pool_has_macos_persona() {
    let pool = PersonaPool::default_pool();
    let mut has_macos = false;

    for i in 0..pool.len() {
        let p = pool
            .clone_persona(i)
            .expect("persona index within pool bounds");
        if p.platform == "MacIntel" {
            has_macos = true;
            break;
        }
    }

    assert!(has_macos, "blueprint Table 6.8.2 requires macOS persona");
}

#[test]
fn pool_rotation_is_round_robin() {
    let mut pool = PersonaPool::default_pool();
    let first = pool
        .clone_persona(0)
        .expect("pool must expose first persona")
        .user_agent; // Since idx starts at 0, first next_persona will get this

    let next_first = pool.next_persona().user_agent;
    assert_eq!(first, next_first);

    // Call 4 more times to cycle completely
    for _ in 0..4 {
        pool.next_persona();
    }

    // Now it should cycle back to the first persona
    let cycled = pool.next_persona().user_agent;
    assert_eq!(first, cycled, "pool must cycle back to start");
}

#[test]
fn all_canvas_seeds_are_unique() {
    let pool = PersonaPool::default_pool();
    let mut seeds = Vec::new();

    for i in 0..pool.len() {
        seeds.push(
            pool.clone_persona(i)
                .expect("persona index within pool bounds")
                .canvas_noise_seed,
        );
    }

    let unique: HashSet<&u64> = seeds.iter().collect();
    assert_eq!(
        unique.len(),
        5,
        "every persona must have a unique seed — D-52"
    );
}

#[test]
fn backward_compat_aliases_produce_correct_chrome_version() {
    let p133 = Persona::chrome_133(99);
    assert_eq!(p133.chrome_version, ChromeProfile::Chrome133);

    let p134 = Persona::chrome_134(100);
    assert_eq!(p134.chrome_version, ChromeProfile::Chrome134);
}

#[test]
fn clone_persona_returns_none_on_out_of_bounds() {
    let pool = PersonaPool::default_pool();
    assert!(pool.clone_persona(999).is_none());
}
