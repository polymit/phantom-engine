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
    pub fn new() -> Result<Self, crate::error::PhantomJsError> {
        use deno_core::{JsRuntime, RuntimeOptions};

        let runtime = JsRuntime::new(RuntimeOptions {
            startup_snapshot: Some(PHANTOM_BASE_SNAPSHOT),
            ..Default::default()
        });

        Ok(Self { runtime })
    }

    /// Execute a JavaScript string synchronously.
    /// Returns the result serialised as JSON string.
    pub fn eval(&mut self, script: &str) -> Result<String, crate::error::PhantomJsError> {
        use deno_core::v8;

        let script_str = script.to_string();
        let result = self.runtime
            .execute_script("<phantom_eval>", script_str)
            .map_err(|e| crate::error::PhantomJsError::JsEvaluation(e.to_string()))?;

        // Convert the v8 Value to a string
        let scope = &mut self.runtime.handle_scope();
        let local = v8::Local::new(scope, result);
        let str_val = local.to_string(scope)
            .map(|s| s.to_rust_string_lossy(scope))
            .unwrap_or_else(|| "undefined".to_string());

        Ok(str_val)
    }

    /// Update the session persona.
    /// Call this after creating a session to override the default persona
    /// that was baked into the snapshot.
    pub fn set_persona(&mut self, persona_json: &str)
        -> Result<(), crate::error::PhantomJsError>
    {
        let script = format!(
            "globalThis.__phantom_persona = Object.assign(\
                globalThis.__phantom_persona || {{}}, {});",
            persona_json
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
