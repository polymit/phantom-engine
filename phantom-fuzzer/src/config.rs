use std::path::PathBuf;

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

/// How hard the planner is allowed to push the blueprint knobs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
pub enum ChaosProfile {
    /// Keeps the same strategies, but caps the worst-case payload size.
    Responsible,
    /// Uses the blueprint's published upper bounds.
    Blueprint,
}

/// Concrete limits used by the planner and mutators.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ChaosLimits {
    pub nesting_min: usize,
    pub nesting_max: usize,
    pub attr_bloat: usize,
    pub zero_width_hits: usize,
    pub max_selector_chain: usize,
    pub max_rpc_blob_bytes: usize,
    pub max_touch_points: usize,
    pub wheel_delta: i64,
    pub timeout_ms: u64,
    pub memory_max_bytes: u64,
    pub cpu_quota_micros: u64,
    pub cpu_period_micros: u64,
    pub tolerance_px: f32,
}

/// Planner config for a single emitted plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuzzerConfig {
    pub corpus_root: PathBuf,
    pub out_root: PathBuf,
    pub plan_count: usize,
    pub rng_seed: u64,
    pub profile: ChaosProfile,
    pub dom_depth: usize,
    pub max_css_rules: usize,
    pub max_js_depth: usize,
}

impl Default for FuzzerConfig {
    fn default() -> Self {
        Self {
            corpus_root: PathBuf::from(".phantom-fuzzer"),
            out_root: PathBuf::from(".phantom-fuzzer/plans"),
            plan_count: 12,
            rng_seed: 1,
            profile: ChaosProfile::Responsible,
            dom_depth: 5,
            max_css_rules: 12,
            max_js_depth: 4,
        }
    }
}

impl ChaosProfile {
    /// Returns the active limits for the selected profile.
    pub fn limits(self) -> ChaosLimits {
        match self {
            Self::Responsible => ChaosLimits {
                nesting_min: 16,
                nesting_max: 128,
                attr_bloat: 256,
                zero_width_hits: 16,
                max_selector_chain: 12,
                max_rpc_blob_bytes: 1 << 20,
                max_touch_points: 16,
                wheel_delta: 32_768,
                timeout_ms: 5_000,
                memory_max_bytes: 512 * 1024 * 1024,
                cpu_quota_micros: 100_000,
                cpu_period_micros: 100_000,
                tolerance_px: 0.5,
            },
            Self::Blueprint => ChaosLimits {
                nesting_min: 1_000,
                nesting_max: 5_000,
                attr_bloat: 10_000,
                zero_width_hits: 256,
                max_selector_chain: 64,
                max_rpc_blob_bytes: 100 * 1024 * 1024,
                max_touch_points: 100,
                wheel_delta: 1_000_000,
                timeout_ms: 5_000,
                memory_max_bytes: 512 * 1024 * 1024,
                cpu_quota_micros: 100_000,
                cpu_period_micros: 100_000,
                tolerance_px: 0.5,
            },
        }
    }
}

impl FuzzerConfig {
    /// Returns the resolved limits for this config.
    pub fn limits(&self) -> ChaosLimits {
        self.profile.limits()
    }
}
