use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use phantom_core::parse_html;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::config::ChaosProfile;
use crate::error::{FuzzerError, Result};

/// Stable seed identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SeedId(String);

impl SeedId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for SeedId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for SeedId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Stable case identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CaseId(String);

impl CaseId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for CaseId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for CaseId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Persisted seed document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Seed {
    pub id: SeedId,
    pub label: String,
    pub html: String,
    pub source: String,
    pub added_at_ms: u64,
}

impl Seed {
    pub fn new(
        label: impl Into<String>,
        html: impl Into<String>,
        source: impl Into<String>,
    ) -> Result<Self> {
        let label = label.into();
        if label.trim().is_empty() {
            return Err(FuzzerError::EmptyLabel);
        }
        let html = html.into();
        let tree = parse_html(&html);
        if tree.document_root.is_none() {
            return Err(FuzzerError::BadSeed(
                "html5ever returned no document root".to_string(),
            ));
        }
        Ok(Self {
            id: SeedId::new(),
            label,
            html,
            source: source.into(),
            added_at_ms: now_ms(),
        })
    }
}

/// High-level case shape emitted by the planner.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CaseKind {
    HtmlMutation,
    Grammar,
    RpcStorm,
}

/// Mutation families tracked in each plan manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MutatorKind {
    InfiniteNesting,
    AttributeBloat,
    TagSplice,
    ZeroWidth,
    CssCascade,
    JsGrammar,
    EventStorm,
    DoubleFetch,
}

/// What the case is trying to break.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TargetKind {
    Panic,
    MemoryGrowth,
    LayoutParity,
    ControlPlane,
    Race,
}

/// Sandbox settings copied from the blueprint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxPlan {
    pub memory_max_bytes: u64,
    pub cpu_quota_micros: u64,
    pub cpu_period_micros: u64,
    pub timeout_ms: u64,
    pub seccomp_profile: String,
}

/// Differential-testing oracle settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OraclePlan {
    pub chromium: bool,
    pub firefox: bool,
    pub safari: bool,
    pub ax_tree: bool,
    pub tolerance_px: f32,
}

/// One JSON-RPC call inside an event storm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcCall {
    pub delay_ms: u64,
    pub method: String,
    pub params: Value,
    pub blob_bytes: Option<usize>,
}

/// Ordered control-plane stress sequence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcStorm {
    pub calls: Vec<RpcCall>,
}

/// One fuzz case plus its execution metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuzzCase {
    pub id: CaseId,
    pub seed_id: Option<SeedId>,
    pub kind: CaseKind,
    pub doc: String,
    pub html: String,
    pub css: String,
    pub js: String,
    pub strategies: Vec<MutatorKind>,
    pub targets: Vec<TargetKind>,
    pub notes: Vec<String>,
    pub storm: Option<RpcStorm>,
    pub sandbox: SandboxPlan,
    pub oracle: OraclePlan,
}

/// Top-level plan manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub id: String,
    pub created_at_ms: u64,
    pub profile: ChaosProfile,
    pub rng_seed: u64,
    pub cases: Vec<FuzzCase>,
}

impl Plan {
    pub fn new(profile: ChaosProfile, rng_seed: u64, cases: Vec<FuzzCase>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            created_at_ms: now_ms(),
            profile,
            rng_seed,
            cases,
        }
    }
}

pub(crate) fn now_ms() -> u64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(delta) => delta.as_millis().min(u64::MAX as u128) as u64,
        Err(_) => 0,
    }
}
