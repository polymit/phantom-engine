// Embed the snapshot at compile time
// OUT_DIR is set by cargo during build
static PHANTOM_BASE_SNAPSHOT: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/PHANTOM_BASE_SNAPSHOT.bin"));

pub struct Tier2Session {
    pub runtime: deno_core::JsRuntime,
}

impl Tier2Session {
    /// Create a new Tier 2 V8 session from the pre-built snapshot.
    ///
    /// The snapshot was created at build time by build.rs.
    /// Loading it takes <5ms — far faster than creating a fresh runtime.
    ///
    /// NEVER call this before init_v8_platform() has been called.
    /// init_v8_platform() must be called in main() before Tokio starts.
    pub fn new(max_heap_bytes: Option<usize>) -> Result<Self, crate::error::PhantomJsError> {
        use deno_core::{JsRuntime, RuntimeOptions, v8};
        use rand::RngCore;

        let create_params = max_heap_bytes
            .map(|limit| v8::CreateParams::default().set_max_old_generation_size_in_bytes(limit));

        let mut runtime = JsRuntime::try_new(RuntimeOptions {
            startup_snapshot: Some(PHANTOM_BASE_SNAPSHOT),
            create_params,
            ..Default::default()
        })
        .map_err(|e| crate::error::PhantomJsError::Internal(e.to_string()))?;

        // SAFETY: Entering the isolate is required to ensure that any handle operations
        // (including those inside JsRuntime::execute_script) are associated with THIS
        // isolate in V8's thread-local state.
        unsafe {
            runtime.v8_isolate().enter();
        }

        let mut rng = rand::rng();
        let seed = rng.next_u64();
        let init_res = runtime.execute_script(
            "<phantom_canvas_seed>",
            format!(
                "globalThis.__phantom_persona = Object.assign(globalThis.__phantom_persona || {{}}, {{ canvas_noise_seed: {}n }});",
                seed
            ),
        );

        unsafe {
            runtime.v8_isolate().exit();
        }

        init_res.map_err(|e| crate::error::PhantomJsError::Internal(e.to_string()))?;

        Ok(Self { runtime })
    }

    /// Execute a JavaScript string synchronously.
    /// Returns the result serialised as JSON string.
    pub fn eval(&mut self, script: &str) -> Result<String, crate::error::PhantomJsError> {
        use deno_core::v8;

        // SAFETY: See new(). We must ensure THIS isolate is current for any handle
        // allocations or clones performed by deno_core::scope!.
        unsafe {
            self.runtime.v8_isolate().enter();
        }

        let res = (|| {
            deno_core::scope!(scope, self.runtime);

            let source = v8::String::new(scope, script).ok_or_else(|| {
                crate::error::PhantomJsError::Internal(
                    "Failed to allocate V8 string for eval".into(),
                )
            })?;

            v8::tc_scope!(let tc_scope, scope);

            let script = v8::Script::compile(tc_scope, source, None).ok_or_else(|| {
                crate::error::PhantomJsError::JsEvaluation("Script compilation failed".into())
            })?;

            let result = script.run(tc_scope).ok_or_else(|| {
                crate::error::PhantomJsError::JsEvaluation("Script execution failed".into())
            })?;

            Ok(result.to_rust_string_lossy(tc_scope))
        })();

        unsafe {
            self.runtime.v8_isolate().exit();
        }
        res
    }

    /// Update the session persona.
    /// Call this after creating a session to override the default persona
    /// that was baked into the snapshot.
    pub fn set_persona(&mut self, persona_json: &str) -> Result<(), crate::error::PhantomJsError> {
        let patch: serde_json::Value = serde_json::from_str(persona_json).map_err(|e| {
            crate::error::PhantomJsError::Internal(format!("invalid persona JSON: {e}"))
        })?;
        let patch_json = serde_json::to_string(&patch).map_err(|e| {
            crate::error::PhantomJsError::Internal(format!("persona serialisation failed: {e}"))
        })?;
        let patch_literal = serde_json::to_string(&patch_json).map_err(|e| {
            crate::error::PhantomJsError::Internal(format!("persona quoting failed: {e}"))
        })?;
        let script = format!(
            "const __patch = JSON.parse({}); \
             globalThis.__phantom_persona = Object.assign(globalThis.__phantom_persona || {{}}, __patch);",
            patch_literal
        );
        self.eval(&script).map(|_| ())
    }

    /// Drop this session and free all V8 resources.
    /// Per D-40: never reuse a session — post-session globals are polluted.
    pub fn destroy(self) {
        drop(self.runtime);
        tracing::debug!("Tier2Session destroyed — V8 isolate freed");
    }
}
