use std::sync::Arc;
use parking_lot::RwLock;
use phantom_core::dom::{DomTree, NodeData};
use std::num::NonZeroUsize;
use rquickjs::{AsyncRuntime, AsyncContext, async_with, prelude::*};


use crate::error::PhantomJsError;

#[derive(Clone)]
#[rquickjs::class]
pub struct PhantomDomHandle {
    pub inner: Arc<RwLock<DomTree>>,
}

impl<'js> rquickjs::class::Trace<'js> for PhantomDomHandle {
    fn trace<'a>(&self, _tracer: rquickjs::class::Tracer<'a, 'js>) {}
}

unsafe impl<'js> rquickjs::JsLifetime<'js> for PhantomDomHandle {
    type Changed<'to> = PhantomDomHandle;
}

impl PhantomDomHandle {
    pub fn new(tree: DomTree) -> Self {
        Self { inner: Arc::new(RwLock::new(tree)) }
    }

    pub fn get_tag_name(&self, arena_id: u64) -> String {
        let tree = self.inner.read();
        let nz = match NonZeroUsize::new(arena_id as usize) {
            Some(n) => n,
            None => return String::new(),
        };
        if let Some(node_id) = tree.arena.get_node_id_at(nz) {
            let node = tree.get(node_id);
            if let NodeData::Element { tag_name, .. } = &node.data {
                return tag_name.clone();
            }
        }
        String::new()
    }

    pub fn get_text_content(&self, arena_id: u64) -> String {
        let tree = self.inner.read();
        let nz = match NonZeroUsize::new(arena_id as usize) {
            Some(n) => n,
            None => return String::new(),
        };
        if let Some(node_id) = tree.arena.get_node_id_at(nz) {
            return tree.get_text_content(node_id);
        }
        String::new()
    }

    pub fn query_selector(&self, selector: &str) -> Option<u64> {
        let tree = self.inner.read();
        tree.query_selector(selector).map(|id| usize::from(id) as u64)
    }

    pub fn query_selector_all(&self, selector: &str) -> Vec<u64> {
        let tree = self.inner.read();
        tree.query_selector_all(selector).into_iter().map(|id| usize::from(id) as u64).collect()
    }

    pub fn get_attribute(&self, arena_id: u64, name: &str) -> Option<String> {
        let tree = self.inner.read();
        let nz = NonZeroUsize::new(arena_id as usize)?;
        let node_id = tree.arena.get_node_id_at(nz)?;
        let node = tree.get(node_id);
        if let NodeData::Element { attributes, .. } = &node.data {
            attributes.get(name).cloned()
        } else {
            None
        }
    }

    pub fn query_selector_from(&self, selector: &str, arena_id: u64) -> Option<u64> {
        let tree = self.inner.read();
        let nz = NonZeroUsize::new(arena_id as usize)?;
        let node_id = tree.arena.get_node_id_at(nz)?;
        tree.query_selector_from(selector, node_id).map(|id| usize::from(id) as u64)
    }

    pub fn create_element(&self, tag: &str) -> u64 {
        let mut tree = self.inner.write();
        let node = phantom_core::dom::DomNode::new(NodeData::Element {
            tag_name: tag.to_string(),
            attributes: std::collections::HashMap::new(),
        });
        let node_id = tree.arena.new_node(node);
        usize::from(node_id) as u64
    }

    pub fn get_title(&self) -> String {
        let tree = self.inner.read();
        tree.get_title()
    }

    pub fn get_element_by_id(&self, id: &str) -> Option<u64> {
        let tree = self.inner.read();
        tree.get_element_by_id(id).map(|id| usize::from(id) as u64)
    }
}

pub struct Tier1Session {
    pub runtime: AsyncRuntime,
    pub context: AsyncContext,
    pub dom_handle: Option<PhantomDomHandle>,
}

impl Tier1Session {
    /// Create a new Tier 1 QuickJS session.
    ///
    /// Sets:
    /// - Memory limit: 50 MB (only works without rust-alloc feature)
    /// - Stack size: 1 MB
    /// - CPU timeout: 10 seconds hard kill via interrupt handler
    ///
    /// Uses AsyncRuntime + AsyncContext — NOT sync Runtime/Context.
    /// All JS execution goes through async_with! macro.
    pub async fn new() -> Result<Self, PhantomJsError> {
        let runtime = AsyncRuntime::new()
            .map_err(|e| PhantomJsError::QuickJsRuntime(e.to_string()))?;

        // 50 MB memory limit
        // This is a NOOP if rust-alloc feature is enabled.
        // Our Cargo.toml intentionally omits rust-alloc. This works.
        runtime.set_memory_limit(50 * 1024 * 1024).await;

        // 1 MB stack limit
        runtime.set_max_stack_size(1024 * 1024).await;

        // Hard 10-second CPU timeout
        // Returns true from the handler = terminate JS execution
        // This is a hard kill — the JS isolate becomes unusable after this
        let start = std::time::Instant::now();
        runtime.set_interrupt_handler(Some(Box::new(move || {
            if start.elapsed().as_millis() > 10_000 {
                tracing::warn!("Tier1Session: CPU budget exceeded — terminating JS");
                return true; // kill
            }
            false
        }))).await;

        // full() loads all standard JS intrinsics (Math, JSON, etc.)
        let context = AsyncContext::full(&runtime)
            .await
            .map_err(|e| PhantomJsError::QuickJsContext(e.to_string()))?;

        Ok(Self {
            runtime,
            context,
            dom_handle: None,
        })
    }

    /// Attach a DOM tree to this session.
    /// Must be called before any JS that touches document.* runs.
    pub async fn attach_dom(&mut self, tree: phantom_core::dom::DomTree) {
        let handle = PhantomDomHandle::new(tree);
        self.dom_handle = Some(handle.clone());

        // Inject the handle into the JS context via store_userdata
        // JS class methods access it via ctx.userdata::<PhantomDomHandle>()
        let h = handle;
        async_with!(self.context => |ctx| {
            ctx.store_userdata(h)
                .expect("store_userdata must not fail");
            Ok::<(), rquickjs::Error>(())
        }).await.expect("attach_dom: async_with must not fail");
    }

    /// Execute a JavaScript string and return the result as a String.
    ///
    /// Uses async_with! — NEVER use ctx.with() in this codebase.
    /// Drains microtasks after execution (required for Promises).
    pub async fn eval(&self, script: &str) -> Result<String, PhantomJsError> {
        let script = script.to_string();
        async_with!(self.context => |ctx| {
            let result = ctx
                .eval::<rquickjs::Value, _>(script)
                .catch(&ctx)
                .map_err(|_| rquickjs::Error::Exception)?;

            // Drain microtask queue — critical for MutationObserver
            // and Promise .then() chains to fire at the right time
            while ctx.execute_pending_job() {}

            // Convert result to string
            let as_str = match result {
                v if v.is_string() => v.as_string().unwrap().to_string()
                    .unwrap_or_else(|_| "undefined".to_string()),
                v if v.is_undefined() => "undefined".to_string(),
                v if v.is_null() => "null".to_string(),
                v if v.is_bool() => if v.as_bool().unwrap() { "true".to_string() } else { "false".to_string() },
                v if v.is_number() => v.as_number().unwrap().to_string(),
                _ => "undefined".to_string(),
            };

            Ok::<String, rquickjs::Error>(as_str)
        })
        .await
        .map_err(|e| PhantomJsError::JsEvaluation(e.to_string()))
    }

    /// Drop this session and free all resources.
    /// After calling this, the session cannot be used.
    /// Per D-08: burn it down — never reuse a session.
    pub fn destroy(self) {
        // Drop order matters: context first, then runtime
        drop(self.context);
        drop(self.runtime);
        tracing::debug!("Tier1Session destroyed — all JS resources freed");
    }
}
