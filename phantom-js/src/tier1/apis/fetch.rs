use rquickjs::{Ctx, Result};

// window.fetch — v0.1 stub
//
// Returns an immediately-resolved Promise (not a real HTTP request).
// Full wreq integration arrives in Phase 3 when phantom-net is wired up.
//
// We build the Promise by looking up the global `Promise.resolve` function
// and calling it directly — no ctx.eval() reentrance required.

pub fn register_fetch<'js>(ctx: &Ctx<'js>, globals: &rquickjs::Object<'js>) -> Result<()> {
    use rquickjs::Function;

    let fetch_fn = Function::new(
        ctx.clone(),
        move |ctx: Ctx<'js>,
              _url: String,
              _opts: rquickjs::prelude::Opt<rquickjs::Value<'js>>|
              -> Result<rquickjs::Value<'js>> {
            // Use a simpler string-based resolve for v0.1
            ctx.eval("Promise.resolve('fetch_stub_response')")
        },
    )?;

    globals.set("fetch", fetch_fn)?;
    Ok(())
}
