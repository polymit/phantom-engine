pub struct MutationBridge {
    context: rquickjs::AsyncContext,
}

impl MutationBridge {
    pub fn new(context: rquickjs::AsyncContext) -> Self {
        Self { context }
    }

    /// Called by Rust whenever the DOM changes.
    /// Fires __phantom_dispatch_mutation() in JS, then drains
    /// the microtask queue so MutationObserver callbacks fire
    /// synchronously (correct browser behaviour per D-42).
    pub async fn notify_mutation(&self, mutation_json: &str) {
        use rquickjs::async_with;
        let mutation_json = mutation_json.to_string();

        let result = async_with!(self.context => |ctx| {
            // Call the JS dispatch function
            let dispatch_call = format!(
                "if (typeof __phantom_dispatch_mutation === 'function') {{ \
                    __phantom_dispatch_mutation({}); \
                }}",
                mutation_json
            );

            ctx.eval::<(), _>(dispatch_call)
                .map_err(|_| rquickjs::Error::Unknown)?;

            // CRITICAL: Drain microtasks immediately after mutation.
            // This ensures MutationObserver callbacks fire before the
            // next CCT serialisation. Per blueprint D-42.
            while ctx.execute_pending_job() {}

            Ok::<(), rquickjs::Error>(())
        })
        .await;

        if let Err(e) = result {
            tracing::warn!("MutationBridge::notify_mutation failed: {:?}", e);
        }
    }
}
