use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::Mutex;
use phantom_anti_detect::{Persona, PersonaPool};
use phantom_core::ParsedPage;
use phantom_js::tier1::pool::Tier1Pool;
use phantom_js::tier2::pool::Tier2Pool;
use phantom_net::SmartNetworkClient;
use phantom_session::SessionBroker;
use std::sync::Once;
use tokio::sync::OnceCell;
use tokio::sync::RwLock;
use uuid::Uuid;

static INIT: Once = Once::new();
static TEST_ADAPTER: OnceCell<&'static EngineAdapter> = OnceCell::const_new();

/// Global V8 platform initialiser. Safe to call multiple times.
pub fn init_v8() {
    INIT.call_once(|| {
        phantom_js::init_v8_platform();
    });
}

/// Returns a shared EngineAdapter instance for testing.
/// This prevents V8 isolate drop order panics by keeping a single set of
/// isolates alive for the duration of the test process via Box::leak.
pub async fn get_test_adapter() -> &'static EngineAdapter {
    TEST_ADAPTER
        .get_or_init(|| async {
            init_v8();
            // ZERO pre-warming for tests to avoid V8 isolate drop order panics across
            // multiple tests. Isolates will be created on-demand and dropped cleanly
            // within each test's lifecycle.
            let adapter = EngineAdapter::new(5, 0, 5, 0).await;
            Box::leak(Box::new(adapter)) as &'static EngineAdapter
        })
        .await
}

/// Wrapper around ParsedPage that opts into Send + Sync.
///
/// TaffyTree (inside LayoutEngine) uses RefCell internally, making it !Send.
/// We only ever access the stored page through a parking_lot::Mutex, which
/// guarantees exclusive access — the RefCell is never touched concurrently.
pub struct SendablePage(pub ParsedPage);

// SAFETY: ParsedPage is only accessed through a Mutex<HashMap<..., SessionPage>>.
// The Mutex serialises all access, so the RefCell inside TaffyTree is never
// accessed from multiple threads simultaneously.
unsafe impl Send for SendablePage {}
unsafe impl Sync for SendablePage {}

/// Per-session snapshot of a navigated page.
/// Stored after each successful navigation so `browser_get_scene_graph`
/// can re-serialise the DOM with different viewport/scroll parameters.
pub struct SessionPage {
    pub page: SendablePage,
    pub url: String,
    pub status: u16,
}

impl SessionPage {
    pub fn new(page: ParsedPage, url: String, status: u16) -> Self {
        Self {
            page: SendablePage(page),
            url,
            status,
        }
    }
}

/// Snapshot of a single browser tab's metadata.
///
/// The tab store does not maintain a live JS context per tab — the active tab
/// maps to whatever page is currently loaded in the shared session pool.
#[derive(Debug, Clone)]
pub struct TabInfo {
    pub id: Uuid,
    pub url: String,
    pub title: String,
    pub active: bool,
}

/// In-memory registry of open tabs for multi-page agent workflows.
#[derive(Debug, Default)]
pub struct TabStore {
    pub tabs: HashMap<Uuid, TabInfo>,
    pub active_tab: Option<Uuid>,
}

/// EngineAdapter is the single shared state type for the MCP server.
/// It owns all subsystems. Clone is cheap — all fields are Arc<T>.
#[derive(Clone)]
pub struct EngineAdapter {
    /// HTTP client built from the first persona in the pool.
    pub network: Arc<SmartNetworkClient>,
    /// Session lifecycle manager.
    pub broker: Arc<SessionBroker>,
    /// Pre-warmed QuickJS session pool (Tier 1).
    pub tier1: Arc<Tier1Pool>,
    /// Pre-warmed V8 session pool (Tier 2).
    pub tier2: Arc<Tier2Pool>,
    /// Persona pool for fingerprint rotation across sessions.
    pub personas: Arc<Mutex<PersonaPool>>,
    /// Per-session page storage for scene graph re-serialisation.
    /// Uses parking_lot::Mutex — lock is never held across .await.
    pub page_store: Arc<Mutex<HashMap<Uuid, SessionPage>>>,
    /// Active page context key used by scene graph/evaluate/click tools.
    /// `None` means the default single-page context (`Uuid::nil()`).
    pub active_page_key: Arc<Mutex<Option<Uuid>>>,
    /// Tab registry for multi-page agent workflows.
    /// Uses tokio::sync::RwLock so callers can hold it safely across awaits.
    pub tab_store: Arc<RwLock<TabStore>>,
    /// Cookie storage.
    pub cookie_store: Arc<tokio::sync::Mutex<cookie_store::CookieStore>>,
    /// Storage manager.
    pub storage: phantom_storage::SessionStorageManager,
    /// Session UUID
    pub session_uuid: uuid::Uuid,
    /// SSE delta broadcast channel
    pub delta_tx: tokio::sync::broadcast::Sender<String>,
}

impl EngineAdapter {
    /// Construct with explicit pool sizes. Pre-warms both JS pools.
    ///
    /// Must be called after `v8::Platform` is initialised and after the Tokio
    /// runtime is started. `Tier1Pool::new()` is async; `Tier2Pool::new()` is sync.
    pub async fn new(t1_max: usize, t1_pre: usize, t2_max: usize, t2_pre: usize) -> Self {
        let mut persona_pool = PersonaPool::default_pool();
        let first_persona = persona_pool.next_persona();
        let network = SmartNetworkClient::with_persona(&first_persona);

        let broker = SessionBroker::new();

        // Tier1Pool::new() already wraps in Arc internally.
        let tier1 = Tier1Pool::new(t1_max, t1_pre).await;

        // Tier2Pool::new() returns Self — we wrap it ourselves.
        let tier2 = Arc::new(Tier2Pool::new(t2_max, t2_pre));

        let (delta_tx, _) = tokio::sync::broadcast::channel(128);

        Self {
            network: Arc::new(network),
            broker: Arc::new(broker),
            tier1,
            tier2,
            personas: Arc::new(Mutex::new(persona_pool)),
            page_store: Arc::new(Mutex::new(HashMap::new())),
            active_page_key: Arc::new(Mutex::new(None)),
            tab_store: Arc::new(RwLock::new(TabStore::default())),
            storage: phantom_storage::SessionStorageManager::new("./storage"),
            session_uuid: uuid::Uuid::new_v4(),
            cookie_store: Arc::new(tokio::sync::Mutex::new(cookie_store::CookieStore::default())),
            delta_tx,
        }
    }

    /// Convenience constructor used by blueprint tests that call `EngineAdapter::new().await`.
    /// Delegates to the 4-arg form with sensible defaults.
    pub async fn new_default() -> Self {
        Self::new(5, 0, 5, 0).await
    }

    pub fn inject_delta(&self, delta: String) -> usize {
        // Sends a delta string to all SSE subscribers.
        // Returns the number of active receivers.
        // Returns 0 if no subscribers (this is fine — not an error).
        self.delta_tx.send(delta).unwrap_or(0)
    }

    /// Rotate to the next persona and return it.
    /// Each new session should call this to vary the fingerprint.
    pub fn next_persona(&self) -> Persona {
        self.personas.lock().next_persona()
    }

    /// Store a navigated page snapshot under the single-session key.
    /// Multi-session keying arrives in a later prompt.
    pub fn store_page(&self, page: SessionPage) {
        let key = (*self.active_page_key.lock()).unwrap_or(Uuid::nil());
        self.page_store.lock().insert(key, page);
    }

    /// Clone the stored ParsedPage for re-serialisation.
    /// Returns None if no page has been navigated to yet.
    pub fn get_page(&self) -> Option<ParsedPage> {
        let key = *self.active_page_key.lock();
        let store = self.page_store.lock();
        match key {
            Some(tab_id) => store.get(&tab_id).map(|sp| sp.page.0.clone()),
            None => store.get(&Uuid::nil()).map(|sp| sp.page.0.clone()),
        }
    }

    /// Get the URL of the currently stored page.
    pub fn get_page_url(&self) -> Option<String> {
        let key = *self.active_page_key.lock();
        let store = self.page_store.lock();
        match key {
            Some(tab_id) => store.get(&tab_id).map(|sp| sp.url.clone()),
            None => store.get(&Uuid::nil()).map(|sp| sp.url.clone()),
        }
    }

    /// Create a new tab, optionally with a URL, and set it as the active tab.
    ///
    /// Navigation is recorded in the tab metadata only — no actual HTTP fetch
    /// is performed here. The caller is responsible for triggering navigation
    /// if real page content is needed.
    pub async fn open_tab(&self, url: Option<String>) -> Uuid {
        let tab_id = Uuid::new_v4();
        let url = url.unwrap_or_default();
        let tab = TabInfo {
            id: tab_id,
            url: url.clone(),
            title: String::new(),
            active: true,
        };

        let mut store = self.tab_store.write().await;
        // Mark all existing tabs as inactive before activating the new one.
        for existing in store.tabs.values_mut() {
            existing.active = false;
        }
        store.tabs.insert(tab_id, tab);
        store.active_tab = Some(tab_id);
        *self.active_page_key.lock() = Some(tab_id);

        tab_id
    }

    /// Switch the active tab to the given ID.
    ///
    /// Returns the `TabInfo` for the newly active tab, or `None` if no tab with
    /// that ID exists.
    pub async fn switch_tab(&self, tab_id: Uuid) -> Option<TabInfo> {
        let mut store = self.tab_store.write().await;
        if !store.tabs.contains_key(&tab_id) {
            return None;
        }
        for tab in store.tabs.values_mut() {
            tab.active = tab.id == tab_id;
        }
        store.active_tab = Some(tab_id);
        *self.active_page_key.lock() = Some(tab_id);
        store.tabs.get(&tab_id).cloned()
    }

    /// Return all tabs in insertion-independent order.
    pub async fn list_tabs(&self) -> Vec<TabInfo> {
        let store = self.tab_store.read().await;
        store.tabs.values().cloned().collect()
    }

    /// Remove a tab from the registry.
    ///
    /// Returns `Some(remaining_count)` on success, `None` if the tab was not found.
    /// If the closed tab was the active tab, the first remaining tab (if any)
    /// is activated automatically.
    pub async fn close_tab(&self, tab_id: Uuid) -> Option<usize> {
        let mut store = self.tab_store.write().await;
        store.tabs.remove(&tab_id)?;
        self.page_store.lock().remove(&tab_id);

        // Activate the first remaining tab if the closed one was active.
        let was_active = store.active_tab == Some(tab_id);
        if was_active {
            store.active_tab = None;
            if let Some(next_id) = store.tabs.keys().next().copied() {
                store.active_tab = Some(next_id);
                if let Some(tab) = store.tabs.get_mut(&next_id) {
                    tab.active = true;
                }
            }
        }
        *self.active_page_key.lock() = store.active_tab;

        Some(store.tabs.len())
    }
}
