use rquickjs::{Ctx, Function, Persistent, Result};
use std::{
    cell::RefCell,
    collections::HashMap,
    sync::atomic::{AtomicU32, Ordering},
};

static TIMER_ID_COUNTER: AtomicU32 = AtomicU32::new(1);

// Thread-local store for pending timer callbacks.
//
// Why thread_local!?  `Persistent<Function<'_>>` is !Send, so it can never
// cross thread boundaries — not in Arc<Mutex<...>>, not in any wrapper.
// thread_local! keeps the value on the runtime thread, where it was created
// and where it will be consumed (inside an async_with! closure).
// spawn_local also runs on that same thread, so the store is always
// accessed by exactly one thread.
thread_local! {
    static TIMER_STORE: RefCell<HashMap<u32, Persistent<Function<'static>>>> =
        RefCell::new(HashMap::new());
}

/// Register setTimeout and clearTimeout on the JS global object.
///
/// Architecture:
/// - `setTimeout(cb, delay)` persists `cb` in a thread-local `HashMap`,
///   keyed by a monotonic timer ID.
/// - A `tokio::task::spawn_local` task sleeps for `delay` ms.  On wake, it
///   re-enters the JS context via `async_with!`, looks up and removes the
///   callback, then calls it.
///
/// Because `spawn_local` runs on the same thread as the QuickJS runtime, the
/// thread-local store is always accessed from the correct thread.  No `Send`
/// bound is required.
///
/// Must be called inside `async_with!` with access to `Ctx`.
pub fn register_timers<'js>(
    ctx: &Ctx<'js>,
    globals: &rquickjs::Object<'js>,
    async_context: rquickjs::AsyncContext,
) -> Result<()> {
    // --- setTimeout ---
    let ctx_clone = async_context.clone();
    let set_timeout = Function::new(
        ctx.clone(),
        move |ctx: Ctx<'js>, callback: Function<'js>, delay_ms: Option<u64>| -> Result<u32> {
            let timer_id = TIMER_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
            let delay = delay_ms.unwrap_or(0);

            // Persist the callback on the current thread.
            let persistent = Persistent::save(&ctx, callback);
            TIMER_STORE.with(|store| {
                store.borrow_mut().insert(timer_id, persistent);
            });

            // Spawn on the LOCAL executor — same thread as the QuickJS runtime.
            // Only a u32 and AsyncContext (Send) cross this boundary.
            let context = ctx_clone.clone();
            tokio::task::spawn_local(async move {
                if delay > 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                }

                // Back on the runtime thread: retrieve and fire the callback.
                rquickjs::async_with!(context => |ctx| {
                    let maybe_cb = TIMER_STORE.with(|store| {
                        store.borrow_mut().remove(&timer_id)
                    });

                    if let Some(persistent) = maybe_cb {
                        if let Ok(cb) = persistent.restore(&ctx) {
                            // RISK-18: best-effort fire — ignore JS errors in callbacks
                            let _ = cb.call::<(), ()>(());
                            // Drain microtasks queued by the callback
                            while ctx.execute_pending_job() {}
                        }
                    }

                    Ok::<(), rquickjs::Error>(())
                })
                .await
                .ok();
            });

            Ok(timer_id)
        },
    )?;
    globals.set("setTimeout", set_timeout)?;

    // --- clearTimeout ---
    let clear_timeout = Function::new(ctx.clone(), |id: Option<u32>| -> Result<()> {
        if let Some(id) = id {
            // Drop the Persistent here — still on the JS thread.
            TIMER_STORE.with(|store| {
                store.borrow_mut().remove(&id);
            });
        }
        Ok(())
    })?;
    globals.set("clearTimeout", clear_timeout)?;

    // setInterval — v0.1: fires once then stops (simplified)
    // Full repeating interval implementation deferred to v0.2
    globals.set("setInterval", globals.get::<_, Function>("setTimeout")?)?;
    globals.set("clearInterval", globals.get::<_, Function>("clearTimeout")?)?;

    Ok(())
}
