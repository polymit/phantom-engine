use rquickjs::{Ctx, Function, Persistent, Result};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc,
    },
};

type SessionTimerStore = HashMap<u64, HashMap<u32, TimerEntry>>;
type SessionCancelledStore = HashMap<u64, HashSet<u32>>;

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
    static TIMER_STORE: RefCell<SessionTimerStore> = RefCell::new(HashMap::new());
    static TIMER_CANCELLED: RefCell<SessionCancelledStore> = RefCell::new(HashMap::new());
}

fn insert_timer(session_id: u64, id: u32, entry: TimerEntry) {
    TIMER_STORE.with(|store| {
        store
            .borrow_mut()
            .entry(session_id)
            .or_default()
            .insert(id, entry);
    });
}
fn remove_timer(session_id: u64, id: u32) -> Option<TimerEntry> {
    TIMER_STORE.with(|store| {
        let mut all = store.borrow_mut();
        let removed = all
            .get_mut(&session_id)
            .and_then(|timers| timers.remove(&id));
        let empty = all.get(&session_id).is_some_and(|timers| timers.is_empty());
        if empty {
            all.remove(&session_id);
        }
        removed
    })
}
fn clear_cancelled(session_id: u64, id: u32) {
    TIMER_CANCELLED.with(|cancelled| {
        let mut all = cancelled.borrow_mut();
        if let Some(ids) = all.get_mut(&session_id) {
            ids.remove(&id);
            if ids.is_empty() {
                all.remove(&session_id);
            }
        }
    });
}
fn take_cancelled(session_id: u64, id: u32) -> bool {
    TIMER_CANCELLED.with(|cancelled| {
        let mut all = cancelled.borrow_mut();
        let Some(ids) = all.get_mut(&session_id) else {
            return false;
        };
        let was_cancelled = ids.remove(&id);
        if ids.is_empty() {
            all.remove(&session_id);
        }
        was_cancelled
    })
}
fn cancel_timer(session_id: u64, id: u32) {
    let removed = remove_timer(session_id, id).is_some();
    if !removed {
        TIMER_CANCELLED.with(|cancelled| {
            cancelled
                .borrow_mut()
                .entry(session_id)
                .or_default()
                .insert(id);
        });
    }
}
fn schedule_timer(
    session_id: u64,
    id: u32,
    delay: u64,
    ctx: rquickjs::AsyncContext,
    session_cancelled: Arc<AtomicBool>,
) -> Result<()> {
    let spawned = catch_unwind(AssertUnwindSafe(|| {
        tokio::task::spawn_local(async move {
            loop {
                if session_cancelled.load(Ordering::SeqCst) {
                    cancel_timer(session_id, id);
                    break;
                }
                if delay > 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                }
                if session_cancelled.load(Ordering::SeqCst) {
                    cancel_timer(session_id, id);
                    break;
                }
                let task_cancelled = Arc::clone(&session_cancelled);
                let keep_running = rquickjs::async_with!(ctx => |ctx| {
                    if task_cancelled.load(Ordering::SeqCst) {
                        cancel_timer(session_id, id);
                        return Ok::<bool, rquickjs::Error>(false);
                    }
                    let Some(entry) = remove_timer(session_id, id) else {
                        return Ok::<bool, rquickjs::Error>(false);
                    };
                    let cancelled = take_cancelled(session_id, id);
                    if let Ok(cb) = entry.callback.restore(&ctx) {
                        // RISK-18: best-effort fire — ignore JS errors in callbacks
                        let _ = cb.call::<(), ()>(());
                        while ctx.execute_pending_job() {}
                        if matches!(entry.kind, TimerKind::Interval) && !cancelled {
                            let next = TimerEntry {
                                callback: Persistent::save(&ctx, cb),
                                kind: TimerKind::Interval,
                            };
                            insert_timer(session_id, id, next);
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
        cancel_timer(session_id, id);
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
    session_id: u64,
    session_cancelled: Arc<AtomicBool>,
) -> Result<()> {
    let timer_ids = Arc::new(AtomicU32::new(1));
    // --- setTimeout ---
    let ctx_clone = async_context.clone();
    let timeout_cancel = Arc::clone(&session_cancelled);
    let timeout_ids = Arc::clone(&timer_ids);
    let set_timeout = Function::new(
        ctx.clone(),
        move |ctx: Ctx<'js>, callback: Function<'js>, delay_ms: Option<u64>| -> Result<u32> {
            let id = timeout_ids.fetch_add(1, Ordering::Relaxed);
            let delay = delay_ms.unwrap_or(0);
            clear_cancelled(session_id, id);
            let entry = TimerEntry {
                callback: Persistent::save(&ctx, callback),
                kind: TimerKind::Once,
            };
            insert_timer(session_id, id, entry);
            schedule_timer(
                session_id,
                id,
                delay,
                ctx_clone.clone(),
                Arc::clone(&timeout_cancel),
            )?;
            Ok(id)
        },
    )?;
    globals.set("setTimeout", set_timeout)?;

    // --- clearTimeout ---
    let clear_timeout = Function::new(ctx.clone(), move |id: Option<u32>| -> Result<()> {
        if let Some(id) = id {
            cancel_timer(session_id, id);
        }
        Ok(())
    })?;
    globals.set("clearTimeout", clear_timeout)?;

    // --- setInterval ---
    let interval_ctx = async_context.clone();
    let interval_cancel = Arc::clone(&session_cancelled);
    let interval_ids = Arc::clone(&timer_ids);
    let set_interval = Function::new(
        ctx.clone(),
        move |ctx: Ctx<'js>, callback: Function<'js>, delay_ms: Option<u64>| -> Result<u32> {
            let id = interval_ids.fetch_add(1, Ordering::Relaxed);
            let delay = delay_ms.unwrap_or(0);
            clear_cancelled(session_id, id);
            let entry = TimerEntry {
                callback: Persistent::save(&ctx, callback),
                kind: TimerKind::Interval,
            };
            insert_timer(session_id, id, entry);
            schedule_timer(
                session_id,
                id,
                delay,
                interval_ctx.clone(),
                Arc::clone(&interval_cancel),
            )?;
            Ok(id)
        },
    )?;
    globals.set("setInterval", set_interval)?;
    globals.set("clearInterval", globals.get::<_, Function>("clearTimeout")?)?;

    Ok(())
}
