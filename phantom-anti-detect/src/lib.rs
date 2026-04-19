use rand::RngCore;
use rand_distr::{Distribution, LogNormal};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromeProfile {
    Chrome133,
    Chrome134,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuProfile {
    WindowsNvidiaRtx3060Ti,
    WindowsAmdRx6600,
    WindowsIntelUhd770,
    MacOsAppleM3Pro,
    MacOsIntelIrisPro,
    WindowsNvidiaRtx4070,
}

impl GpuProfile {
    pub fn strings(self) -> (&'static str, &'static str) {
        match self {
            Self::WindowsNvidiaRtx3060Ti => (
                "Google Inc. (NVIDIA)",
                "ANGLE (NVIDIA, NVIDIA GeForce RTX 3060 Ti Direct3D11 vs_5_0 ps_5_0, D3D11)",
            ),
            Self::WindowsAmdRx6600 => (
                "Google Inc. (AMD)",
                "ANGLE (AMD, AMD Radeon RX 6600 Direct3D11 vs_5_0 ps_5_0, D3D11)",
            ),
            Self::WindowsIntelUhd770 => (
                "Google Inc. (Intel)",
                "ANGLE (Intel, Intel(R) UHD Graphics 770 Direct3D11 vs_5_0 ps_5_0, D3D11)",
            ),
            Self::MacOsAppleM3Pro => (
                "Google Inc.",
                "ANGLE (Apple, ANGLE Metal Renderer: Apple M3 Pro, Unspecified Version)",
            ),
            Self::MacOsIntelIrisPro => (
                "Google Inc.",
                "ANGLE (Intel Inc., ANGLE Metal Renderer: Intel Iris Pro Graphics, Unspecified Version)",
            ),
            Self::WindowsNvidiaRtx4070 => (
                "Google Inc. (NVIDIA)",
                "ANGLE (NVIDIA, NVIDIA GeForce RTX 4070 Direct3D11 vs_5_0 ps_5_0, D3D11)",
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Persona {
    pub user_agent: String,
    pub platform: String,
    pub chrome_version: ChromeProfile,
    pub screen_width: u32,
    pub screen_height: u32,
    pub device_pixel_ratio: f32,
    pub hardware_concurrency: u32,
    pub device_memory: u32,
    pub language: String,
    pub languages: Vec<String>,
    pub timezone: String,
    pub webgl_vendor: String,
    pub webgl_renderer: String,
    pub canvas_noise_seed: u64,
    pub platform_version: String,
    pub ua_full_version: String,
    pub ua_architecture: String,
    pub ua_bitness: String,
    pub ua_wow64: bool,
    pub ua_platform: String,
    pub chrome_major: String,
}

impl Persona {
    pub fn win11_chrome133_nvidia_rtx3060ti(seed: u64) -> Self {
        let (vendor, renderer) = GpuProfile::WindowsNvidiaRtx3060Ti.strings();
        Self {
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.6943.98 Safari/537.36".into(),
            platform: "Win32".into(),
            chrome_version: ChromeProfile::Chrome133,
            screen_width: 1920,
            screen_height: 1080,
            device_pixel_ratio: 1.0,
            hardware_concurrency: 8,
            device_memory: 8,
            language: "en-US".into(),
            languages: vec!["en-US".into(), "en".into()],
            timezone: "America/New_York".into(),
            webgl_vendor: vendor.into(),
            webgl_renderer: renderer.into(),
            canvas_noise_seed: seed,
            platform_version: "15.0.0".into(),
            ua_full_version: "133.0.6943.98".into(),
            ua_architecture: "x86".into(),
            ua_bitness: "64".into(),
            ua_wow64: false,
            ua_platform: "Windows".into(),
            chrome_major: "133".into(),
        }
    }

    pub fn win11_chrome133_amd_rx6600(seed: u64) -> Self {
        let (vendor, renderer) = GpuProfile::WindowsAmdRx6600.strings();
        Self {
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.6943.98 Safari/537.36".into(),
            platform: "Win32".into(),
            chrome_version: ChromeProfile::Chrome133,
            screen_width: 2560,
            screen_height: 1440,
            device_pixel_ratio: 1.5,
            hardware_concurrency: 12,
            device_memory: 8,
            language: "en-US".into(),
            languages: vec!["en-US".into(), "en".into()],
            timezone: "America/Chicago".into(),
            webgl_vendor: vendor.into(),
            webgl_renderer: renderer.into(),
            canvas_noise_seed: seed,
            platform_version: "15.0.0".into(),
            ua_full_version: "133.0.6943.98".into(),
            ua_architecture: "x86".into(),
            ua_bitness: "64".into(),
            ua_wow64: false,
            ua_platform: "Windows".into(),
            chrome_major: "133".into(),
        }
    }

    pub fn win10_chrome133_intel_uhd770(seed: u64) -> Self {
        let (vendor, renderer) = GpuProfile::WindowsIntelUhd770.strings();
        Self {
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.6943.98 Safari/537.36".into(),
            platform: "Win32".into(),
            chrome_version: ChromeProfile::Chrome133,
            screen_width: 1920,
            screen_height: 1080,
            device_pixel_ratio: 1.0,
            hardware_concurrency: 6,
            device_memory: 4,
            language: "en-US".into(),
            languages: vec!["en-US".into(), "en".into()],
            timezone: "America/Los_Angeles".into(),
            webgl_vendor: vendor.into(),
            webgl_renderer: renderer.into(),
            canvas_noise_seed: seed,
            platform_version: "10.0.0".into(),
            ua_full_version: "133.0.6943.98".into(),
            ua_architecture: "x86".into(),
            ua_bitness: "64".into(),
            ua_wow64: false,
            ua_platform: "Windows".into(),
            chrome_major: "133".into(),
        }
    }

    pub fn win11_chrome134_nvidia_rtx3060ti(seed: u64) -> Self {
        let (vendor, renderer) = GpuProfile::WindowsNvidiaRtx3060Ti.strings();
        Self {
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.6998.36 Safari/537.36".into(),
            platform: "Win32".into(),
            chrome_version: ChromeProfile::Chrome134,
            screen_width: 1920,
            screen_height: 1080,
            device_pixel_ratio: 1.0,
            hardware_concurrency: 8,
            device_memory: 8,
            language: "en-US".into(),
            languages: vec!["en-US".into(), "en".into()],
            timezone: "America/Chicago".into(),
            webgl_vendor: vendor.into(),
            webgl_renderer: renderer.into(),
            canvas_noise_seed: seed,
            platform_version: "15.0.0".into(),
            ua_full_version: "134.0.6998.36".into(),
            ua_architecture: "x86".into(),
            ua_bitness: "64".into(),
            ua_wow64: false,
            ua_platform: "Windows".into(),
            chrome_major: "134".into(),
        }
    }

    pub fn macos_sonoma_chrome133_m3pro(seed: u64) -> Self {
        let (vendor, renderer) = GpuProfile::MacOsAppleM3Pro.strings();
        Self {
            user_agent: "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.6943.98 Safari/537.36".into(),
            platform: "MacIntel".into(),
            chrome_version: ChromeProfile::Chrome133,
            screen_width: 2560,
            screen_height: 1600,
            device_pixel_ratio: 2.0,
            hardware_concurrency: 8,
            device_memory: 8,
            language: "en-US".into(),
            languages: vec!["en-US".into(), "en".into()],
            timezone: "America/New_York".into(),
            webgl_vendor: vendor.into(),
            webgl_renderer: renderer.into(),
            canvas_noise_seed: seed,
            platform_version: "14.0.0".into(),
            ua_full_version: "133.0.6943.98".into(),
            ua_architecture: "arm".into(),
            ua_bitness: "64".into(),
            ua_wow64: false,
            ua_platform: "macOS".into(),
            chrome_major: "133".into(),
        }
    }

    pub fn chrome_133(seed: u64) -> Self {
        Self::win11_chrome133_nvidia_rtx3060ti(seed)
    }

    pub fn chrome_134(seed: u64) -> Self {
        Self::win11_chrome134_nvidia_rtx3060ti(seed)
    }
}

#[derive(Debug, Clone)]
pub struct PersonaPool {
    personas: Vec<Persona>,
    idx: usize,
}

impl PersonaPool {
    pub fn new(personas: Vec<Persona>) -> Self {
        let personas = if personas.is_empty() {
            let mut rng = rand::rng();
            vec![Persona::chrome_133(next_seed(&mut rng))]
        } else {
            personas
        };
        Self { personas, idx: 0 }
    }

    pub fn default_pool() -> Self {
        let mut rng = rand::rng();
        Self::default_pool_with_rng(&mut rng)
    }

    fn default_pool_with_rng<R: RngCore + ?Sized>(rng: &mut R) -> Self {
        let s0 = next_seed(rng);
        let s1 = next_distinct_seed(rng, s0);
        let s2 = next_distinct_seed(rng, s1);
        let s3 = next_distinct_seed(rng, s2);
        let s4 = next_distinct_seed(rng, s3);
        Self::new(vec![
            Persona::win11_chrome133_nvidia_rtx3060ti(s0),
            Persona::win11_chrome133_amd_rx6600(s1),
            Persona::win10_chrome133_intel_uhd770(s2),
            Persona::win11_chrome134_nvidia_rtx3060ti(s3),
            Persona::macos_sonoma_chrome133_m3pro(s4),
        ])
    }

    pub fn len(&self) -> usize {
        self.personas.len()
    }

    pub fn is_empty(&self) -> bool {
        self.personas.is_empty()
    }

    pub fn next_persona(&mut self) -> Persona {
        let p = self.personas[self.idx].clone();
        self.idx = (self.idx + 1) % self.personas.len();
        p
    }

    pub fn clone_persona(&self, idx: usize) -> Option<Persona> {
        self.personas.get(idx).cloned()
    }
}

fn next_seed<R: RngCore + ?Sized>(rng: &mut R) -> u64 {
    rng.next_u64()
}

fn next_distinct_seed<R: RngCore + ?Sized>(rng: &mut R, current: u64) -> u64 {
    let mut next = next_seed(rng);
    if next == current {
        next = next_seed(rng);
        if next == current {
            next = next.wrapping_add(1);
        }
    }
    next
}

/// Timing profile used by higher-level behavior systems.
#[derive(Debug, Clone)]
pub struct BehaviorTiming {
    click_hesitation: LogNormal<f64>,
    inter_action: LogNormal<f64>,
}

impl BehaviorTiming {
    #[allow(clippy::expect_used)]
    pub fn new() -> Self {
        Self {
            click_hesitation: LogNormal::new(4.2, 0.9).expect("valid lognormal params"),
            inter_action: LogNormal::new(5.8, 1.1).expect("valid lognormal params"),
        }
    }

    pub fn click_hesitation_ms(&self) -> u64 {
        let mut rng = rand::rng();
        self.click_hesitation.sample(&mut rng).clamp(20.0, 500.0) as u64
    }

    pub fn inter_action_delay_ms(&self) -> u64 {
        let mut rng = rand::rng();
        self.inter_action.sample(&mut rng).clamp(50.0, 3000.0) as u64
    }
}

impl Default for BehaviorTiming {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{BehaviorTiming, ChromeProfile, Persona, PersonaPool};
    use rand::RngCore;

    struct MockStepRng {
        val: u64,
        step: u64,
    }

    impl MockStepRng {
        fn new(val: u64, step: u64) -> Self {
            Self { val, step }
        }
    }

    impl RngCore for MockStepRng {
        fn next_u32(&mut self) -> u32 {
            self.next_u64() as u32
        }
        fn next_u64(&mut self) -> u64 {
            let v = self.val;
            self.val = self.val.wrapping_add(self.step);
            v
        }
        fn fill_bytes(&mut self, _dest: &mut [u8]) {
            unimplemented!()
        }
    }

    #[test]
    fn persona_pool_rotates_round_robin() {
        let mut pool = PersonaPool::new(vec![Persona::chrome_133(10), Persona::chrome_134(11)]);

        let p1 = pool.next_persona();
        let p2 = pool.next_persona();
        let p3 = pool.next_persona();

        assert_eq!(p1.chrome_version, ChromeProfile::Chrome133);
        assert_eq!(p2.chrome_version, ChromeProfile::Chrome134);
        assert_eq!(p3.chrome_version, ChromeProfile::Chrome133);
    }

    #[test]
    fn behavior_timing_stays_in_expected_range() {
        let t = BehaviorTiming::new();
        let click = t.click_hesitation_ms();
        let delay = t.inter_action_delay_ms();
        assert!((20..=500).contains(&click));
        assert!((50..=3000).contains(&delay));
    }

    #[test]
    fn default_pool_uses_rng_output_for_canvas_seed() {
        let mut rng = MockStepRng::new(41, 17); // 41, 58, 75, ...
        let mut pool = PersonaPool::default_pool_with_rng(&mut rng);

        let p1 = pool.next_persona();
        let p2 = pool.next_persona();

        assert_eq!(p1.chrome_version, ChromeProfile::Chrome133);
        assert_eq!(p2.chrome_version, ChromeProfile::Chrome133); // Second is AMD 133
        assert_eq!(p1.canvas_noise_seed, 41);
        assert_eq!(p2.canvas_noise_seed, 58);
    }

    #[test]
    fn default_pool_seeds_are_distinct_even_with_repeating_rng() {
        let mut rng = MockStepRng::new(7, 0); // always 7
        let mut pool = PersonaPool::default_pool_with_rng(&mut rng);

        let p1 = pool.next_persona();
        let p2 = pool.next_persona();

        assert_ne!(p1.canvas_noise_seed, p2.canvas_noise_seed);
        assert_eq!(p1.canvas_noise_seed, 7);
        assert_eq!(p2.canvas_noise_seed, 8);
    }
}
