use rand::Rng;
use rand_distr::{Distribution, LogNormal};

pub struct BehaviorEngine {
    /// Click hesitation delay — time between mousedown and mouseup
    /// LogNormal(μ=4.2, σ=0.9) → median ≈ 66.7ms
    click_hesitation: LogNormal<f64>,

    /// Delay between distinct user actions
    /// LogNormal(μ=5.8, σ=1.1) → median ≈ 330ms
    inter_action: LogNormal<f64>,

    /// Delay between keystrokes when typing
    /// LogNormal(μ=4.8, σ=0.7) → median ≈ 121ms per character
    char_typing_delay: LogNormal<f64>,
}

impl BehaviorEngine {
    pub fn new() -> Self {
        Self {
            // EXACT parameters from blueprint — DO NOT CHANGE
            click_hesitation: LogNormal::new(4.2, 0.9)
                .expect("click_hesitation LogNormal params must stay valid"),
            inter_action: LogNormal::new(5.8, 1.1)
                .expect("inter_action LogNormal params must stay valid"),
            char_typing_delay: LogNormal::new(4.8, 0.7)
                .expect("char_typing_delay LogNormal params must stay valid"),
        }
    }

    /// Generate a cubic Bezier mouse path from `from` to `to`.
    /// Returns 20–40 intermediate points along the curve.
    /// The curve has randomised control points creating natural arcing.
    ///
    /// This matches the EXACT formula from blueprint section 6.8.5.
    pub fn generate_mouse_path(&self, from: (f64, f64), to: (f64, f64)) -> Vec<(f64, f64)> {
        let mut rng = rand::thread_rng();

        // Two control points with jitter — creates natural arc
        // Formula from blueprint section 6.8.5 — exact
        let cx1 = from.0 + (to.0 - from.0) * 0.25 + self.jitter(&mut rng, 20.0);
        let cy1 = from.1 + (to.1 - from.1) * 0.10 + self.jitter(&mut rng, 30.0);
        let cx2 = from.0 + (to.0 - from.0) * 0.75 + self.jitter(&mut rng, 20.0);
        let cy2 = from.1 + (to.1 - from.1) * 0.90 + self.jitter(&mut rng, 30.0);

        // 20–40 sampled points along the curve
        let n = 20 + (rng.r#gen::<u8>() % 20) as usize;

        (0..=n)
            .map(|i| {
                let t = i as f64 / n as f64;
                let mt = 1.0 - t;
                // Cubic Bezier formula — exact from blueprint
                (
                    mt * mt * mt * from.0
                        + 3.0 * mt * mt * t * cx1
                        + 3.0 * mt * t * t * cx2
                        + t * t * t * to.0,
                    mt * mt * mt * from.1
                        + 3.0 * mt * mt * t * cy1
                        + 3.0 * mt * t * t * cy2
                        + t * t * t * to.1,
                )
            })
            .collect()
    }

    /// Sample a click hesitation delay in milliseconds.
    /// This is the delay between mousedown and mouseup events.
    pub fn click_hesitation_ms(&self) -> u64 {
        let sample = self.click_hesitation.sample(&mut rand::thread_rng());
        // Clamp to 20ms–500ms — extremely short or long delays are anomalous
        sample.clamp(20.0, 500.0) as u64
    }

    /// Sample a delay between two distinct actions in milliseconds.
    pub fn inter_action_delay_ms(&self) -> u64 {
        let sample = self.inter_action.sample(&mut rand::thread_rng());
        sample.clamp(50.0, 3000.0) as u64
    }

    /// Sample a per-character typing delay in milliseconds.
    pub fn char_typing_delay_ms(&self) -> u64 {
        let sample = self.char_typing_delay.sample(&mut rand::thread_rng());
        sample.clamp(30.0, 500.0) as u64
    }

    fn jitter(&self, rng: &mut impl Rng, scale: f64) -> f64 {
        (rng.r#gen::<f64>() - 0.5) * scale
    }
}

impl Default for BehaviorEngine {
    fn default() -> Self {
        Self::new()
    }
}
