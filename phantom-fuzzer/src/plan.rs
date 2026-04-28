use std::fs;
use std::path::{Path, PathBuf};

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde_json::json;

use crate::config::FuzzerConfig;
use crate::corpus::Corpus;
use crate::error::{map_io, Result};
use crate::grammar::build_doc;
use crate::model::{
    CaseId, CaseKind, FuzzCase, MutatorKind, OraclePlan, Plan, RpcCall, RpcStorm, SandboxPlan,
    Seed, TargetKind,
};
use crate::mutate::mutate_seed;

/// Planner that turns a corpus into a plan manifest.
#[derive(Debug, Clone)]
pub struct Planner {
    cfg: FuzzerConfig,
}

impl Planner {
    /// Creates a planner from config.
    pub fn new(cfg: FuzzerConfig) -> Self {
        Self { cfg }
    }

    /// Builds a plan from an in-memory seed list.
    pub fn build(&self, seeds: &[Seed]) -> Result<Plan> {
        if seeds.is_empty() {
            return Err(crate::error::FuzzerError::EmptyCorpus);
        }

        let mut rng = StdRng::seed_from_u64(self.cfg.rng_seed);
        let mut cases = Vec::with_capacity(self.cfg.plan_count);

        for idx in 0..self.cfg.plan_count {
            let case = match idx % 3 {
                0 => self.build_mutation_case(&mut rng, seeds)?,
                1 => self.build_grammar_case(&mut rng),
                _ => self.build_storm_case(&mut rng),
            };
            cases.push(case);
        }

        Ok(Plan::new(self.cfg.profile, self.cfg.rng_seed, cases))
    }

    fn build_mutation_case(&self, rng: &mut StdRng, seeds: &[Seed]) -> Result<FuzzCase> {
        let seed = &seeds[rng.random_range(0..seeds.len())];
        let payload = mutate_seed(
            rng,
            seed,
            self.cfg.limits(),
            self.cfg.max_css_rules,
            self.cfg.max_js_depth,
        )?;

        Ok(FuzzCase {
            id: CaseId::new(),
            seed_id: Some(seed.id.clone()),
            kind: CaseKind::HtmlMutation,
            doc: payload.doc,
            html: payload.html,
            css: payload.css,
            js: payload.js,
            strategies: payload.strategies,
            targets: vec![TargetKind::Panic, TargetKind::MemoryGrowth],
            notes: payload.notes,
            storm: None,
            sandbox: sandbox(self.cfg.limits()),
            oracle: oracle(self.cfg.limits()),
        })
    }

    fn build_grammar_case(&self, rng: &mut StdRng) -> FuzzCase {
        let payload = build_doc(
            rng,
            self.cfg.limits(),
            self.cfg.dom_depth,
            self.cfg.max_css_rules,
            self.cfg.max_js_depth,
        );

        FuzzCase {
            id: CaseId::new(),
            seed_id: None,
            kind: CaseKind::Grammar,
            doc: payload.doc,
            html: payload.html,
            css: payload.css,
            js: payload.js,
            strategies: payload.strategies,
            targets: vec![TargetKind::Panic, TargetKind::LayoutParity],
            notes: payload.notes,
            storm: None,
            sandbox: sandbox(self.cfg.limits()),
            oracle: oracle(self.cfg.limits()),
        }
    }

    fn build_storm_case(&self, rng: &mut StdRng) -> FuzzCase {
        let payload = build_doc(
            rng,
            self.cfg.limits(),
            self.cfg.dom_depth,
            self.cfg.max_css_rules,
            self.cfg.max_js_depth,
        );
        let storm = RpcStorm {
            calls: vec![
                RpcCall {
                    delay_ms: 0,
                    method: "browser_navigate".to_string(),
                    params: json!({ "url": "https://example.com" }),
                    blob_bytes: None,
                },
                RpcCall {
                    delay_ms: 50,
                    method: "browser_session_snapshot".to_string(),
                    params: json!({}),
                    blob_bytes: None,
                },
                RpcCall {
                    delay_ms: 55,
                    method: "browser_close_tab".to_string(),
                    params: json!({ "tab_id": "{{active_tab_id}}" }),
                    blob_bytes: None,
                },
                RpcCall {
                    delay_ms: 60,
                    method: "browser_type".to_string(),
                    params: json!({ "selector": "#email", "text": "$blob" }),
                    blob_bytes: Some(self.cfg.limits().max_rpc_blob_bytes),
                },
            ],
        };

        FuzzCase {
            id: CaseId::new(),
            seed_id: None,
            kind: CaseKind::RpcStorm,
            doc: payload.doc,
            html: payload.html,
            css: payload.css,
            js: payload.js,
            strategies: vec![
                MutatorKind::CssCascade,
                MutatorKind::JsGrammar,
                MutatorKind::EventStorm,
                MutatorKind::DoubleFetch,
            ],
            targets: vec![TargetKind::ControlPlane, TargetKind::Race],
            notes: vec![
                "plan-only control-plane storm".to_string(),
                "close_tab uses placeholder active tab id".to_string(),
            ],
            storm: Some(storm),
            sandbox: sandbox(self.cfg.limits()),
            oracle: oracle(self.cfg.limits()),
        }
    }
}

/// Writes plans and payloads to disk.
#[derive(Debug, Clone)]
pub struct PlanWriter {
    root: PathBuf,
}

impl PlanWriter {
    /// Creates a writer rooted at `root`.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Writes one plan and returns the created directory.
    pub fn write(&self, plan: &Plan) -> Result<PathBuf> {
        let dir = self.root.join(format!("plan-{}", plan.id));
        let cases_dir = dir.join("cases");
        let meta_dir = dir.join("meta");

        for next in [&dir, &cases_dir, &meta_dir] {
            map_io(next, fs::create_dir_all(next))?;
        }

        let manifest = dir.join("manifest.json");
        let buf = serde_json::to_vec_pretty(plan)?;
        map_io(&manifest, fs::write(&manifest, buf))?;

        for case in &plan.cases {
            self.write_case(&cases_dir, &meta_dir, case)?;
        }

        Ok(dir)
    }

    fn write_case(&self, cases_dir: &Path, meta_dir: &Path, case: &FuzzCase) -> Result<()> {
        let html_path = cases_dir.join(format!("{}.html", case.id.as_str()));
        let css_path = cases_dir.join(format!("{}.css", case.id.as_str()));
        let js_path = cases_dir.join(format!("{}.js", case.id.as_str()));
        let meta_path = meta_dir.join(format!("{}.json", case.id.as_str()));

        map_io(&html_path, fs::write(&html_path, &case.doc))?;
        map_io(&css_path, fs::write(&css_path, &case.css))?;
        map_io(&js_path, fs::write(&js_path, &case.js))?;
        let buf = serde_json::to_vec_pretty(case)?;
        map_io(&meta_path, fs::write(&meta_path, buf))?;
        Ok(())
    }
}

fn sandbox(limits: crate::config::ChaosLimits) -> SandboxPlan {
    SandboxPlan {
        memory_max_bytes: limits.memory_max_bytes,
        cpu_quota_micros: limits.cpu_quota_micros,
        cpu_period_micros: limits.cpu_period_micros,
        timeout_ms: limits.timeout_ms,
        seccomp_profile: "{\"defaultAction\":\"SCMP_ACT_KILL\",\"syscalls\":[{\"names\":[\"read\",\"write\",\"openat\",\"close\",\"epoll_wait\",\"mmap\",\"munmap\",\"futex\"],\"action\":\"SCMP_ACT_ALLOW\"},{\"names\":[\"execve\",\"fork\",\"vfork\",\"ptrace\"],\"action\":\"SCMP_ACT_KILL\"}]}".to_string(),
    }
}

fn oracle(limits: crate::config::ChaosLimits) -> OraclePlan {
    OraclePlan {
        chromium: true,
        firefox: true,
        safari: true,
        ax_tree: true,
        tolerance_px: limits.tolerance_px,
    }
}

/// Reads the corpus at `root`, builds one plan, and writes it out.
pub fn plan_once(cfg: FuzzerConfig) -> Result<PathBuf> {
    let corpus = Corpus::open(cfg.corpus_root.clone());
    let seeds = corpus.load_seeds()?;
    let planner = Planner::new(cfg.clone());
    let plan = planner.build(&seeds)?;
    PlanWriter::new(cfg.out_root).write(&plan)
}
