use rand_distr::{Distribution, LogNormal};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromeProfile {
    Chrome133,
    Chrome134,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Persona {
    pub user_agent: String,
    pub platform: String,
    pub chrome_version: ChromeProfile,
    pub hardware_concurrency: u32,
    pub device_memory: u32,
    pub language: String,
    pub languages: Vec<String>,
    pub timezone: String,
    pub webgl_vendor: String,
    pub webgl_renderer: String,
    pub canvas_noise_seed: u64,
}

impl Persona {
    pub fn chrome_133(seed: u64) -> Self {
        Self {
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.6943.98 Safari/537.36".to_string(),
            platform: "Win32".to_string(),
            chrome_version: ChromeProfile::Chrome133,
            hardware_concurrency: 8,
            device_memory: 8,
            language: "en-US".to_string(),
            languages: vec!["en-US".to_string(), "en".to_string()],
            timezone: "America/New_York".to_string(),
            webgl_vendor: "Google Inc. (NVIDIA)".to_string(),
            webgl_renderer: "ANGLE (NVIDIA, NVIDIA GeForce RTX 3060)".to_string(),
            canvas_noise_seed: seed,
        }
    }

    pub fn chrome_134(seed: u64) -> Self {
        Self {
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.6998.36 Safari/537.36".to_string(),
            platform: "Win32".to_string(),
            chrome_version: ChromeProfile::Chrome134,
            hardware_concurrency: 8,
            device_memory: 8,
            language: "en-US".to_string(),
            languages: vec!["en-US".to_string(), "en".to_string()],
            timezone: "America/Chicago".to_string(),
            webgl_vendor: "Google Inc. (NVIDIA)".to_string(),
            webgl_renderer: "ANGLE (NVIDIA, NVIDIA GeForce RTX 3060)".to_string(),
            canvas_noise_seed: seed,
        }
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
            vec![Persona::chrome_133(1)]
        } else {
            personas
        };
        Self { personas, idx: 0 }
    }

    pub fn default_pool() -> Self {
        Self::new(vec![Persona::chrome_133(1), Persona::chrome_134(2)])
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
}

/// Timing profile used by higher-level behavior systems.
#[derive(Debug, Clone)]
pub struct BehaviorTiming {
    click_hesitation: LogNormal<f64>,
    inter_action: LogNormal<f64>,
}

impl BehaviorTiming {
    pub fn new() -> Self {
        Self {
            click_hesitation: LogNormal::new(4.2, 0.9).expect("valid lognormal params"),
            inter_action: LogNormal::new(5.8, 1.1).expect("valid lognormal params"),
        }
    }

    pub fn click_hesitation_ms(&self) -> u64 {
        let mut rng = rand::thread_rng();
        self.click_hesitation.sample(&mut rng).clamp(20.0, 500.0) as u64
    }

    pub fn inter_action_delay_ms(&self) -> u64 {
        let mut rng = rand::thread_rng();
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
}
