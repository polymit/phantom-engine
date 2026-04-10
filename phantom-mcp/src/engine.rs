use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use parking_lot::Mutex;
use phantom_anti_detect::{Persona, PersonaPool};
use phantom_core::{rebuild_page_from_tree, DomTree, ParsedPage};
use phantom_js::tier1::pool::Tier1Pool;
use phantom_js::tier2::pool::Tier2Pool;
use phantom_net::SmartNetworkClient;
use phantom_serializer::CctDelta;
use phantom_session::{SessionBroker, SessionState};
use phantom_storage::SessionStorageManager;
use std::sync::Once;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::OnceCell;
use tokio::sync::RwLock;
use uuid::Uuid;

static INIT: Once = Once::new();
static TEST_ADAPTER: OnceCell<&'static EngineAdapter> = OnceCell::const_new();
const DELTA_REPLAY_CAP: usize = 256;

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

/// Per-session snapshot of a navigated page.
/// Stored after each successful navigation so `browser_get_scene_graph`
/// can re-serialise the DOM with different viewport/scroll parameters.
pub struct SessionPage {
    pub tree: DomTree,
    pub url: String,
    pub status: u16,
    pub viewport_width: f32,
    pub viewport_height: f32,
}

impl SessionPage {
    pub fn new(page: ParsedPage, url: String, status: u16) -> Self {
        Self::with_viewport(page, url, status, 1280.0, 720.0)
    }

    pub fn with_viewport(
        page: ParsedPage,
        url: String,
        status: u16,
        viewport_width: f32,
        viewport_height: f32,
    ) -> Self {
        Self {
            tree: page.tree,
            url,
            status,
            viewport_width,
            viewport_height,
        }
    }

    pub fn to_parsed_page(&self) -> Option<ParsedPage> {
        rebuild_page_from_tree(
            self.tree.clone(),
            &self.url,
            self.viewport_width,
            self.viewport_height,
        )
        .ok()
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
    /// Replay buffer for late SSE subscribers.
    /// Keeps the most recent deltas when no receiver is attached.
    pub delta_replay: Arc<Mutex<VecDeque<String>>>,
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
            delta_replay: Arc::new(Mutex::new(VecDeque::with_capacity(DELTA_REPLAY_CAP))),
        }
    }

    /// Convenience constructor used by blueprint tests that call `EngineAdapter::new().await`.
    /// Delegates to the 4-arg form with sensible defaults.
    pub async fn new_default() -> Self {
        Self::new(5, 0, 5, 0).await
    }

    pub fn inject_delta(&self, delta: String) -> usize {
        {
            let mut replay = self.delta_replay.lock();
            if replay.len() >= DELTA_REPLAY_CAP {
                replay.pop_front();
            }
            replay.push_back(delta.clone());
        }

        match self.delta_tx.send(delta) {
            Ok(receivers) => receivers,
            Err(err) => {
                tracing::debug!("delta queued with no active SSE subscriber: {}", err.0);
                0
            }
        }
    }

    /// Sends a typed CCT delta to SSE subscribers.
    pub fn inject_cct_delta(&self, delta: CctDelta) -> usize {
        self.inject_delta(delta.to_string())
    }

    /// Snapshot the replay buffer for diagnostics and tests.
    pub fn delta_replay_snapshot(&self) -> Vec<String> {
        self.delta_replay.lock().iter().cloned().collect()
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
            Some(tab_id) => store.get(&tab_id).and_then(SessionPage::to_parsed_page),
            None => store
                .get(&Uuid::nil())
                .and_then(SessionPage::to_parsed_page),
        }
    }

    /// Clone the stored ParsedPage and its viewport metadata.
    /// Returns None if no page has been navigated to yet.
    pub fn get_page_with_viewport(&self) -> Option<(ParsedPage, String, f32, f32)> {
        let key = *self.active_page_key.lock();
        let store = self.page_store.lock();
        let page = match key {
            Some(tab_id) => store.get(&tab_id),
            None => store.get(&Uuid::nil()),
        }?;

        let parsed = page.to_parsed_page()?;
        Some((
            parsed,
            page.url.clone(),
            page.viewport_width,
            page.viewport_height,
        ))
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
        let next_active = store.active_tab;
        let remaining = store.tabs.len();

        // Keep lock order aligned with get_page()/get_page_url() to avoid
        // transient mismatches and lock inversion.
        let mut active_page_key = self.active_page_key.lock();
        let mut page_store = self.page_store.lock();
        page_store.remove(&tab_id);
        *active_page_key = next_active;

        Some(remaining)
    }

    pub async fn suspend(&self, session_id: Uuid) -> Result<String, String> {
        let start = Instant::now();
        let session_id_str = session_id.to_string();

        // STEP 1 — Collect cookies
        let cookies_json = {
            let store = self.cookie_store.lock().await; // tokio Mutex OK across await
            serde_json::to_vec(&*store).map_err(|e| e.to_string())?
        }; // lock dropped here before any further awaits

        // STEP 2 — Collect localStorage from disk
        let storage2 = self.storage.clone();
        let sid2 = session_id_str.clone();
        let local_storage =
            tokio::task::spawn_blocking(move || collect_localstorage(&sid2, &storage2))
                .await
                .map_err(|e| e.to_string())?;

        // STEP 3 — Collect IndexedDB bytes
        let storage3 = self.storage.clone();
        let sid3 = session_id_str.clone();
        let indexeddb = tokio::task::spawn_blocking(move || collect_indexeddb(&sid3, &storage3))
            .await
            .map_err(|e| e.to_string())?;

        // STEP 4 — Build snapshot
        let data = phantom_storage::snapshot::SnapshotData {
            session_id: session_id_str.clone(),
            cookies_json,
            local_storage,
            indexeddb,
            cache_blobs: HashMap::new(), // cache blobs handled in future prompt
            cache_meta: None,
        };
        let compressed =
            tokio::task::spawn_blocking(move || phantom_storage::snapshot::build_snapshot(&data))
                .await
                .map_err(|e| e.to_string())?
                .map_err(|e| e.to_string())?;

        // STEP 5 — Write to disk
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs();

        let storage5 = self.storage.clone();
        let sid5 = session_id_str.clone();
        let session_dir = tokio::task::spawn_blocking(move || storage5.create_session_dir(&sid5))
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?;

        let path = session_dir.join(format!("snapshot-{}-{}.tar.zst", session_id_str, timestamp));
        tokio::fs::write(&path, &compressed)
            .await
            .map_err(|e| e.to_string())?;

        // STEP 6 — Update SessionBroker state
        self.broker
            .set_state(session_id, SessionState::Suspended)
            .map_err(|e| e.to_string())?;

        let elapsed = start.elapsed();
        if elapsed.as_millis() >= 200 {
            tracing::warn!(
                "suspend elapsed: {}ms (target: < 200ms)",
                elapsed.as_millis()
            );
        }
        tracing::info!("suspend elapsed: {}ms", elapsed.as_millis());

        Ok(path.to_string_lossy().into_owned())
    }
}

fn collect_localstorage(
    session_id_str: &str,
    storage: &SessionStorageManager,
) -> HashMap<String, Vec<u8>> {
    let mut result = HashMap::new();
    if let Ok(dir) = storage.localstorage_dir(session_id_str) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("sled") {
                    if let Some(hash) = path.file_stem().and_then(|s| s.to_str()) {
                        if let Ok(db) = sled::open(&path) {
                            let mut map = HashMap::new();
                            for item in db.iter() {
                                if let Ok((k, v)) = item {
                                    map.insert(
                                        String::from_utf8_lossy(&k).into_owned(),
                                        String::from_utf8_lossy(&v).into_owned(),
                                    );
                                }
                            }
                            if let Ok(json_bytes) = serde_json::to_vec(&map) {
                                result.insert(hash.to_string(), json_bytes);
                            }
                        }
                    }
                }
            }
        }
    }
    result
}

fn collect_indexeddb(
    session_id_str: &str,
    storage: &SessionStorageManager,
) -> HashMap<String, Vec<u8>> {
    let mut result = HashMap::new();
    if let Ok(dir) = storage.indexeddb_dir(session_id_str) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("sqlite") {
                    if let Some(hash) = path.file_stem().and_then(|s| s.to_str()) {
                        if let Ok(bytes) = std::fs::read(&path) {
                            result.insert(hash.to_string(), bytes);
                        }
                    }
                }
            }
        }
    }
    result
}
