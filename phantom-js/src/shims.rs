use rquickjs::embed;
use rquickjs::loader::Bundle;

/// Pre-compiled QuickJS bytecode bundle.
/// These JS files are compiled to bytecode at BUILD TIME by the
/// embed! macro. Loading them at runtime costs ~0.1ms — no parsing.
///
/// Files must exist at these paths relative to the crate root.
/// embed! verifies this at compile time.
pub static PHANTOM_SHIMS: Bundle = embed! {
    "phantom/shims":             "js/browser_shims.js",
    "phantom/event_target":      "js/event_target.js",
    "phantom/mutation_observer": "js/mutation_observer.js",
    "phantom/location":          "js/location.js",
};

/// Load the shim bundle into a QuickJS context.
/// Must be called BEFORE any page JS runs.
/// Must be called AFTER ctx.store_userdata() calls.
pub async fn load_shims(
    context: &rquickjs::AsyncContext,
    persona_json: &str,
) -> Result<(), crate::error::PhantomJsError> {
    use rquickjs::async_with;

    let persona_json = persona_json.to_string();

    async_with!(context => |ctx| {
        // Step 1: Inject __phantom_persona before shims load
        // The shims reference __phantom_persona.* — it must exist first
        let inject_persona = format!(
            "globalThis.__phantom_persona = {};",
            persona_json
        );
        ctx.eval::<(), _>(inject_persona)
            .map_err(|_| rquickjs::Error::Unknown)?;

        // Step 2: Import each shim module
        // Modules are pre-compiled bytecode from the embed! bundle
        rquickjs::Module::import(&ctx, "phantom/shims")?
            .into_future::<()>()
            .await
            .map_err(|_| rquickjs::Error::Unknown)?;

        rquickjs::Module::import(&ctx, "phantom/event_target")?
            .into_future::<()>()
            .await
            .map_err(|_| rquickjs::Error::Unknown)?;

        rquickjs::Module::import(&ctx, "phantom/mutation_observer")?
            .into_future::<()>()
            .await
            .map_err(|_| rquickjs::Error::Unknown)?;

        rquickjs::Module::import(&ctx, "phantom/location")?
            .into_future::<()>()
            .await
            .map_err(|_| rquickjs::Error::Unknown)?;

        Ok::<(), rquickjs::Error>(())
    })
    .await
    .map_err(|e| crate::error::PhantomJsError::ShimInjection(e.to_string()))
}
