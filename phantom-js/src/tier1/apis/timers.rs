use rquickjs::{Ctx, Function, Persistent, Result};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    panic::{catch_unwind, AssertUnwindSafe},
    sync::atomic::{AtomicU32, Ordering},
};

static TIMER_ID_COUNTER: AtomicU32 = AtomicU32::new(1);
#[derive(Copy, Clone)]
enum TimerKind {
    Once,
    Interval,
}
struct TimerEntry {
    callback: Persistent<Function<'static>>,
    kind: TimerKind,
}

// Thread-local store for pending timer callbacks.
//
// Why thread_local!?  `Persistent<Function<'_>>` is !Send, so it can never
// cross thread boundaries — not in Arc<Mutex<...>>, not in any wrapper.
// thread_local! keeps the value on the runtime thread, where it was created
// and where it will be consumed (inside an async_with! closure).
// spawn_local also runs on that same thread, so the store is always
// accessed by exactly one thread.
thread_local! {
    static TIMER_STORE: RefCell<HashMap<u32, TimerEntry>> = RefCell::new(HashMap::new());
    static TIMER_CANCELLED: RefCell<HashSet<u32>> = RefCell::new(HashSet::new());
}

fn cancel_timer(id: u32) {
    let removed = TIMER_STORE.with(|store| store.borrow_mut().remove(&id).is_some());
    if !removed {
        TIMER_CANCELLED.with(|cancelled| {
            cancelled.borrow_mut().insert(id);
        });
    }
}
fn schedule_timer(id: u32, delay: u64, ctx: rquickjs::AsyncContext) -> Result<()> {
    let spawned = catch_unwind(AssertUnwindSafe(|| {
        tokio::task::spawn_local(async move {
            loop {
                if delay > 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                }
                let keep_running = rquickjs::async_with!(ctx => |ctx| {
                    let Some(entry) = TIMER_STORE.with(|store| store.borrow_mut().remove(&id)) else {
                        return Ok::<bool, rquickjs::Error>(false);
                    };
                    let cancelled = TIMER_CANCELLED.with(|cancelled| cancelled.borrow_mut().remove(&id));
                    if let Ok(cb) = entry.callback.restore(&ctx) {
                        // RISK-18: best-effort fire — ignore JS errors in callbacks
                        let _ = cb.call::<(), ()>(());
                        while ctx.execute_pending_job() {}
                        if matches!(entry.kind, TimerKind::Interval) && !cancelled {
                            let next = TimerEntry {
                                callback: Persistent::save(&ctx, cb),
                                kind: TimerKind::Interval,
                            };
                            TIMER_STORE.with(|store| {
                                store.borrow_mut().insert(id, next);
                            });
                            return Ok::<bool, rquickjs::Error>(true);
                        }
                    }
                    Ok::<bool, rquickjs::Error>(false)
                })
                .await
                .unwrap_or(false);
                if !keep_running {
                    break;
                }
            }
        });
    }));
    if spawned.is_err() {
        cancel_timer(id);
        return Err(rquickjs::Error::Exception);
    }
    Ok(())
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
            let id = TIMER_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
            let delay = delay_ms.unwrap_or(0);
            TIMER_CANCELLED.with(|cancelled| {
                cancelled.borrow_mut().remove(&id);
            });
            let entry = TimerEntry {
                callback: Persistent::save(&ctx, callback),
                kind: TimerKind::Once,
            };
            TIMER_STORE.with(|store| {
                store.borrow_mut().insert(id, entry);
            });
            schedule_timer(id, delay, ctx_clone.clone())?;
            Ok(id)
        },
    )?;
    globals.set("setTimeout", set_timeout)?;

    // --- clearTimeout ---
    let clear_timeout = Function::new(ctx.clone(), |id: Option<u32>| -> Result<()> {
        if let Some(id) = id {
            cancel_timer(id);
        }
        Ok(())
    })?;
    globals.set("clearTimeout", clear_timeout)?;

    // --- setInterval ---
    let interval_ctx = async_context.clone();
    let set_interval = Function::new(
        ctx.clone(),
        move |ctx: Ctx<'js>, callback: Function<'js>, delay_ms: Option<u64>| -> Result<u32> {
            let id = TIMER_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
            let delay = delay_ms.unwrap_or(0);
            TIMER_CANCELLED.with(|cancelled| {
                cancelled.borrow_mut().remove(&id);
            });
            let entry = TimerEntry {
                callback: Persistent::save(&ctx, callback),
                kind: TimerKind::Interval,
            };
            TIMER_STORE.with(|store| {
                store.borrow_mut().insert(id, entry);
            });
            schedule_timer(id, delay, interval_ctx.clone())?;
            Ok(id)
        },
    )?;
    globals.set("setInterval", set_interval)?;
    globals.set("clearInterval", globals.get::<_, Function>("clearTimeout")?)?;

    Ok(())
}
