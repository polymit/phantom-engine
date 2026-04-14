use rquickjs::{Ctx, Function, Object, Result, prelude::This};

// window.fetch — v0.1 stub
//
// Returns an immediately-resolved Promise (not a real HTTP request).
// Full wreq integration arrives in Phase 3 when phantom-net is wired up.
//
// We build the Promise by looking up the global `Promise.resolve` function
// and calling it directly — no ctx.eval() reentrance required.

pub fn register_fetch<'js>(ctx: &Ctx<'js>, globals: &rquickjs::Object<'js>) -> Result<()> {
    let fetch_fn = Function::new(
        ctx.clone(),
        move |ctx: Ctx<'js>,
              _url: String,
              _opts: rquickjs::prelude::Opt<rquickjs::Value<'js>>|
              -> Result<rquickjs::Value<'js>> {
            let globals = ctx.globals();
            let promise_ctor: Object<'js> = globals.get("Promise")?;
            let resolve: Function<'js> = promise_ctor.get("resolve")?;

            let response = Object::new(ctx.clone())?;
            response.set("status", 200)?;
            response.set("ok", true)?;
            response.set("statusText", "OK")?;

            let ctx_inner = ctx.clone();
            let promise_inner = promise_ctor.clone();
            let resolve_inner = resolve.clone();
            let text_fn = Function::new(ctx.clone(), move || -> Result<rquickjs::Value<'js>> {
                resolve_inner.call((
                    This(promise_inner.clone()),
                    rquickjs::String::from_str(ctx_inner.clone(), "")?,
                ))
            })?;
            response.set("text", text_fn)?;

            let ctx_inner = ctx.clone();
            let promise_inner = promise_ctor.clone();
            let resolve_inner = resolve.clone();
            let json_fn = Function::new(ctx.clone(), move || -> Result<rquickjs::Value<'js>> {
                resolve_inner.call((This(promise_inner.clone()), Object::new(ctx_inner.clone())?))
            })?;
            response.set("json", json_fn)?;

            resolve.call((This(promise_ctor), response))
        },
    )?;

    globals.set("fetch", fetch_fn)?;
    Ok(())
}
