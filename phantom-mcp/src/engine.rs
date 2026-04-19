use std::collections::{HashMap, VecDeque};
use std::io::{BufReader, Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
use std::sync::Arc;

use crate::metrics;
use parking_lot::Mutex;
use phantom_anti_detect::{Persona, PersonaPool};
use phantom_core::{
    rebuild_page_from_tree, BrowserError, BrowserSessionError, DomTree, ParsedPage,
};
use phantom_js::tier1::pool::Tier1Pool;
use phantom_js::tier2::pool::Tier2Pool;
use phantom_net::SmartNetworkClient;
use phantom_serializer::CctDelta;
use phantom_session::{EngineKind, ResourceBudget, Session, SessionBroker, SessionState};
use phantom_storage::SessionStorageManager;
use sha2::{Digest, Sha256};
use std::sync::Once;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::OnceCell;
use tokio::sync::RwLock;
use tokio::task::spawn_blocking;
use tracing::Instrument;
use uuid::Uuid;

static INIT: Once = Once::new();
static TEST_ADAPTER: OnceCell<Arc<EngineAdapter>> = OnceCell::const_new();
const DELTA_REPLAY_CAP: usize = 256;
const MAX_BLOCKING_THREADS: usize = 32;

/// Global V8 platform initialiser. Safe to call multiple times.
pub fn init_v8() {
    INIT.call_once(|| {
        phantom_js::init_v8_platform();
    });
}

/// Returns a shared EngineAdapter instance for testing.
/// This uses an Arc to allow clean teardown when the test process ends.
pub async fn get_test_adapter() -> Arc<EngineAdapter> {
    TEST_ADAPTER
        .get_or_init(|| async {
            init_v8();
            Arc::new(EngineAdapter::new(5, 0, 5, 0, ResourceBudget::default()).await)
        })
        .await
        .clone()
}

/// Per-session snapshot of a navigated page.
/// Stored after each successful navigation so `browser_get_scene_graph`
/// can re-serialise the DOM with different viewport/scroll parameters.
#[derive(Clone)]
pub struct SessionPage {
    pub tree: DomTree,
    pub url: String,
    pub status: u16,
    pub viewport_width: f32,
    pub viewport_height: f32,
}

impl SessionPage {
    pub fn new(tree: DomTree, url: String, status: u16) -> Self {
        Self::with_viewport(tree, url, status, 1280.0, 720.0)
    }

    pub fn with_viewport(
        tree: DomTree,
        url: String,
        status: u16,
        viewport_width: f32,
        viewport_height: f32,
    ) -> Self {
        Self {
            tree,
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
/// It owns all subsystems. Clone is cheap — all fields are `Arc<T>`.
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
    /// Semaphore to serialize tool execution and prevent session state contention.
    pub session_lock: Arc<tokio::sync::Semaphore>,
    /// Global semaphore to prevent OS thread exhaustion from blocking tasks.
    pub blocking_limit: Arc<tokio::sync::Semaphore>,
    /// Creation time for measuring session duration.
    pub created_at: Instant,
    /// Tracks whether the primary session is still alive for metrics accounting.
    pub session_active: Arc<AtomicBool>,
}

impl EngineAdapter {
    /// Construct with explicit pool sizes. Pre-warms both JS pools.
    ///
    /// Must be called after `v8::Platform` is initialised and after the Tokio
    /// runtime is started. `Tier1Pool::new()` is async; `Tier2Pool::new()` is sync.
    pub async fn new(
        t1_max: usize,
        t1_pre: usize,
        t2_max: usize,
        t2_pre: usize,
        budget: ResourceBudget,
    ) -> Self {
        let mut persona_pool = PersonaPool::default_pool();
        let first_persona = persona_pool.next_persona();
        let persona_id = first_persona.user_agent.clone();
        let mut network = SmartNetworkClient::with_persona(&first_persona);
        network.max_network_bytes = Some(budget.max_network_bytes);
        let session_budget = budget.clone();
        let session_uuid = uuid::Uuid::new_v4();

        let broker = SessionBroker::new();

        // Tier1Pool::new() already wraps in Arc internally.
        let tier1 = Tier1Pool::new(t1_max, t1_pre).await;

        // Tier2Pool::new() returns Self — we wrap it ourselves.
        let tier2 = Arc::new(Tier2Pool::new(t2_max, t2_pre, Some(budget.max_heap_bytes)));

        let (delta_tx, _) = tokio::sync::broadcast::channel(128);

        let engine = Self {
            network: Arc::new(network),
            broker: Arc::new(broker),
            tier1,
            tier2,
            personas: Arc::new(Mutex::new(persona_pool)),
            page_store: Arc::new(Mutex::new(HashMap::new())),
            active_page_key: Arc::new(Mutex::new(None)),
            tab_store: Arc::new(RwLock::new(TabStore::default())),
            storage: phantom_storage::SessionStorageManager::new("./storage"),
            session_uuid,
            cookie_store: Arc::new(tokio::sync::Mutex::new(cookie_store::CookieStore::default())),
            delta_tx,
            delta_replay: Arc::new(Mutex::new(VecDeque::with_capacity(DELTA_REPLAY_CAP))),
            session_lock: Arc::new(tokio::sync::Semaphore::new(1)),
            blocking_limit: Arc::new(tokio::sync::Semaphore::new(MAX_BLOCKING_THREADS)),
            created_at: Instant::now(),
            session_active: Arc::new(AtomicBool::new(true)),
        };

        metrics::SESSIONS_ACTIVE.inc();
        metrics::SESSIONS_CREATED_TOTAL
            .with_label_values(&["tier1"])
            .inc();
        metrics::CIRCUIT_BREAKER_STATE
            .with_label_values(&["tier1"])
            .set(0);
        metrics::CIRCUIT_BREAKER_STATE
            .with_label_values(&["tier2"])
            .set(0);

        phantom_storage::install_storage_quota_observer(Arc::new(|bytes| {
            metrics::STORAGE_QUOTA_USED_BYTES.set(bytes);
        }));

        let session = Session::with_uuid(
            session_uuid,
            EngineKind::QuickJs,
            session_budget,
            persona_id,
        );
        engine.broker.register(session);
        let _ = engine.broker.set_state(session_uuid, SessionState::Running);

        engine
    }

    pub async fn new_default() -> Self {
        Self::new(5, 0, 5, 0, ResourceBudget::default()).await
    }
}

impl Drop for EngineAdapter {
    fn drop(&mut self) {
        if self.session_active.swap(false, AtomicOrdering::AcqRel) {
            metrics::SESSIONS_ACTIVE.dec();
        }
        let duration = self.created_at.elapsed().as_secs_f64();
        metrics::SESSION_DURATION_SECONDS.observe(duration);
    }
}

impl EngineAdapter {
    pub fn session_count(&self) -> usize {
        self.broker.len()
    }

    pub fn enforce_budget_usage(
        &self,
        heap_bytes: usize,
        cpu_ms: u64,
        network_bytes: usize,
    ) -> Result<(), BrowserError> {
        match self.broker.record_usage_and_check(
            self.session_uuid,
            heap_bytes,
            cpu_ms,
            network_bytes,
        ) {
            Ok(()) => Ok(()),
            Err(phantom_session::SessionError::BudgetExceeded {
                resource,
                used,
                limit,
            }) => {
                tracing::warn!(
                    session_id = %self.session_uuid,
                    resource = %resource,
                    used,
                    limit,
                    "session budget exceeded; destroying session"
                );
                let _ = self.broker.remove(self.session_uuid);
                if self.session_active.swap(false, AtomicOrdering::AcqRel) {
                    metrics::SESSIONS_ACTIVE.dec();
                }
                Err(BrowserError::Session(BrowserSessionError::BudgetExceeded {
                    resource,
                    used,
                    limit,
                }))
            }
            Err(err) => Err(err.into()),
        }
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

    /// Snapshot the currently active page key used for navigation writes.
    pub fn current_page_key(&self) -> Uuid {
        (*self.active_page_key.lock()).unwrap_or(Uuid::nil())
    }

    /// Store a page only if the active key still matches the expected key.
    ///
    /// Returns false when active context changed while navigation was in flight.
    pub fn store_page_if_current(&self, expected_key: Uuid, page: SessionPage) -> bool {
        let active_page_key = self.active_page_key.lock();
        let mut page_store = self.page_store.lock();
        let current_key = (*active_page_key).unwrap_or(Uuid::nil());
        if current_key != expected_key {
            return false;
        }
        page_store.insert(current_key, page);
        true
    }

    /// Clone the stored ParsedPage for re-serialisation.
    /// Returns None if no page has been navigated to yet.
    pub async fn get_page(&self) -> Option<ParsedPage> {
        let page = {
            let key = *self.active_page_key.lock();
            let store = self.page_store.lock();
            match key {
                Some(tab_id) => store.get(&tab_id).cloned(),
                None => store.get(&Uuid::nil()).cloned(),
            }
        }?;

        let limit = self.blocking_limit.clone();
        spawn_blocking(move || {
            let _permit = limit.try_acquire().ok();
            page.to_parsed_page()
        })
        .await
        .ok()
        .flatten()
    }

    /// Clone the stored ParsedPage and its viewport metadata.
    /// Returns None if no page has been navigated to yet.
    pub async fn get_page_with_viewport(&self) -> Option<(ParsedPage, String, f32, f32)> {
        let page = {
            let key = *self.active_page_key.lock();
            let store = self.page_store.lock();
            match key {
                Some(tab_id) => store.get(&tab_id).cloned(),
                None => store.get(&Uuid::nil()).cloned(),
            }
        }?;

        let limit = self.blocking_limit.clone();
        let page_cloned = page.clone();
        let parsed = spawn_blocking(move || {
            let _permit = limit.try_acquire().ok();
            page_cloned.to_parsed_page()
        })
        .await
        .ok()
        .flatten()?;

        Some((parsed, page.url, page.viewport_width, page.viewport_height))
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
        let span = tracing::info_span!(
            "session.suspend",
            session_id = %session_id,
            snapshot_path = tracing::field::Empty,
            elapsed_ms = tracing::field::Empty
        );
        async move {
            let start = Instant::now();
            let session_id_str = session_id.to_string();

            // Session cookies have no Expires/Max-Age — serde's Serialize impl
            // filters them out. cookie_store::serde::json preserves them.
            let cookies_json = {
                let store = self.cookie_store.lock().await;
                let mut buf = Vec::new();
                cookie_store::serde::json::save_incl_expired_and_nonpersistent(&store, &mut buf)
                    .map_err(|e| e.to_string())?;
                buf
            }; // tokio Mutex guard dropped here

            // STEP 2 — Collect localStorage from disk
            let storage2 = self.storage.clone();
            let sid2 = session_id_str.clone();
            let limit2 = self.blocking_limit.clone();
            let local_storage = tokio::task::spawn_blocking(move || {
                let _permit = limit2.try_acquire().ok();
                collect_localstorage(&sid2, &storage2)
            })
            .await
            .map_err(|e| e.to_string())?;

            // STEP 3 — Collect IndexedDB bytes
            let storage3 = self.storage.clone();
            let sid3 = session_id_str.clone();
            let limit3 = self.blocking_limit.clone();
            let indexeddb = tokio::task::spawn_blocking(move || {
                let _permit = limit3.try_acquire().ok();
                collect_indexeddb(&sid3, &storage3)
            })
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
            let limit4 = self.blocking_limit.clone();
            let compressed = tokio::task::spawn_blocking(move || {
                let _permit = limit4.try_acquire().ok();
                phantom_storage::snapshot::build_snapshot(&data)
            })
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?;

            // STEP 5 — Write to disk
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_millis();

            let storage5 = self.storage.clone();
            let sid5 = session_id_str.clone();
            let limit5 = self.blocking_limit.clone();
            let session_dir = tokio::task::spawn_blocking(move || {
                let _permit = limit5.try_acquire().ok();
                storage5.create_session_dir(&sid5)
            })
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?;

            let path =
                session_dir.join(format!("snapshot-{}-{}.tar.zst", session_id_str, timestamp));
            tokio::fs::write(&path, &compressed)
                .await
                .map_err(|e| e.to_string())?;

            metrics::STORAGE_QUOTA_USED_BYTES.set(compressed.len() as i64);

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

            tracing::Span::current().record("snapshot_path", path.to_string_lossy().as_ref());
            tracing::Span::current().record("elapsed_ms", elapsed.as_millis() as u64);

            Ok(path.to_string_lossy().into_owned())
        }
        .instrument(span)
        .await
    }

    /// Rehydrates session state from the latest snapshot archive on disk.
    /// Verifies HMAC integrity before loading any data — rejects tampered snapshots.
    pub async fn resume(&self, session_id: Uuid) -> Result<(), String> {
        let span = tracing::info_span!(
            "session.resume",
            session_id = %session_id,
            elapsed_ms = tracing::field::Empty,
            hmac_verified = tracing::field::Empty
        );
        async move {
            let start = Instant::now();
            let session_id_str = session_id.to_string();

            let session_dir = self
                .storage
                .session_dir(&session_id_str)
                .map_err(|e| e.to_string())?;

            let snapshot_path = Self::find_latest_snapshot(&session_dir)?;

            let compressed = tokio::fs::read(&snapshot_path)
                .await
                .map_err(|e| e.to_string())?;

            // HMAC gate — reject before touching any in-memory state
            let manifest = phantom_storage::snapshot::read_manifest_from_snapshot(&compressed)
                .map_err(|e| e.to_string())?;
            phantom_storage::snapshot::verify_manifest(&manifest).map_err(|e| e.to_string())?;

            tracing::Span::current().record("hmac_verified", true);

            // BATCH REHYDRATION — Consolidate ALL CPU/IO intensive tasks into ONE blocking task.
            // This includes extraction, verification, and database writes.
            let storage = self.storage.clone();
            let sid = session_id_str.clone();
            let manifest_clone = manifest.clone();
            let limit_resume = self.blocking_limit.clone();
            let compressed_clone = compressed.clone();

            let cookies_bytes = tokio::task::spawn_blocking(move || {
                let _permit = limit_resume.try_acquire().ok();

                // 1. Extract files (CPU intensive)
                let snapshot_files = Self::extract_files_from_snapshot(&compressed_clone)?;

                // 2. Verify hashes (CPU intensive)
                Self::verify_snapshot_payload(&manifest_clone, &snapshot_files)?;

                // 3. Rehydrate storage (IO intensive)
                let session_dir = storage.session_dir(&sid).map_err(|e| e.to_string())?;
                for filename in manifest_clone.checksums.keys() {
                    // localStorage
                    if let Some(rest) = filename.strip_prefix("localstorage/") {
                        let hash = rest.strip_suffix(".json").ok_or("invalid ls filename")?;
                        let json_bytes = snapshot_files.get(filename).cloned().ok_or_else(|| {
                            format!("snapshot missing file: {}", filename)
                        })?;
                        if json_bytes.is_empty() { continue; }

                        let kv_map: HashMap<String, String> = serde_json::from_slice(&json_bytes)
                            .map_err(|e| format!("localstorage deserialise: {}", e))?;

                        let ls_dir = session_dir.join("localstorage");
                        std::fs::create_dir_all(&ls_dir).map_err(|e| e.to_string())?;
                        let db = sled::open(ls_dir.join(format!("{}.sled", hash)))
                            .map_err(|e| e.to_string())?;
                        db.clear().map_err(|e| e.to_string())?;
                        for (k, v) in &kv_map {
                            db.insert(k.as_bytes(), v.as_bytes()).map_err(|e| e.to_string())?;
                        }
                    }
                    // IndexedDB
                    else if let Some(rest) = filename.strip_prefix("indexeddb/") {
                        let hash = rest.strip_suffix(".sqlite").ok_or("invalid idb filename")?;
                        let sqlite_bytes = snapshot_files.get(filename).cloned().ok_or_else(|| {
                            format!("snapshot missing file: {}", filename)
                        })?;
                        if sqlite_bytes.is_empty() { continue; }

                        let idb_dir = session_dir.join("indexeddb");
                        std::fs::create_dir_all(&idb_dir).map_err(|e| e.to_string())?;
                        std::fs::write(idb_dir.join(format!("{}.sqlite", hash)), &sqlite_bytes)
                            .map_err(|e| e.to_string())?;
                    }
                }

                // Return cookies for async rehydration
                Ok::<_, String>(snapshot_files.get("cookies.bin").cloned())
            })
            .await
            .map_err(|e| e.to_string())??;

            // 4. Rehydrate cookies (requires async mutex)
            if let Some(bytes) = cookies_bytes {
                if !bytes.is_empty() {
                    let store = cookie_store::serde::json::load_all(BufReader::new(Cursor::new(&bytes)))
                        .map_err(|e| format!("cookie deserialise: {}", e))?;
                    *self.cookie_store.lock().await = store;
                }
            }

            self.broker
                .set_state(session_id, SessionState::Running)
                .map_err(|e| e.to_string())?;

            let elapsed = start.elapsed();
            if elapsed.as_millis() >= 50 {
                tracing::warn!("resume elapsed: {}ms (target: < 50ms)", elapsed.as_millis());
            }
            tracing::info!("resume elapsed: {}ms", elapsed.as_millis());

            tracing::Span::current().record("elapsed_ms", elapsed.as_millis() as u64);

            Ok(())
        }
        .instrument(span)
        .await
    }

    fn verify_snapshot_payload(
        manifest: &phantom_storage::snapshot::SnapshotManifest,
        files: &HashMap<String, Vec<u8>>,
    ) -> Result<(), String> {
        for (filename, expected_hash) in &manifest.checksums {
            let file_bytes = files
                .get(filename)
                .ok_or_else(|| format!("snapshot missing file: {}", filename))?;

            if let Some(expected_size) = manifest.sizes.get(filename) {
                if *expected_size != file_bytes.len() as u64 {
                    return Err(format!("snapshot size mismatch for {}", filename));
                }
            } else {
                return Err(format!("manifest missing size for {}", filename));
            }

            let mut hasher = Sha256::new();
            hasher.update(file_bytes);
            let actual_hash = hex::encode(hasher.finalize());
            if &actual_hash != expected_hash {
                return Err(format!("snapshot checksum mismatch for {}", filename));
            }
        }

        Ok(())
    }

    /// Scans a session directory for `.tar.zst` snapshots and returns the most
    /// recently modified one.
    ///
    /// We intentionally sort by filesystem mtime first so clock skew in the
    /// embedded filename timestamp cannot roll sessions back to stale snapshots.
    fn find_latest_snapshot(session_dir: &Path) -> Result<PathBuf, String> {
        let entries = std::fs::read_dir(session_dir).map_err(|e| e.to_string())?;

        let mut snapshots: Vec<(PathBuf, u128, u128)> = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("zst")
                && path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .is_some_and(|n| n.ends_with(".tar.zst"))
            {
                if let Ok(meta) = path.metadata() {
                    if let Ok(modified) = meta.modified() {
                        let file_name = path
                            .file_name()
                            .and_then(|s| s.to_str())
                            .unwrap_or_default();

                        // Numeric suffix in snapshot filenames:
                        // - suspend: snapshot-<secs>.tar.zst
                        // - clone:   snapshot-<uuid>-<millis>.tar.zst
                        // Used only as a tie-breaker for equal mtimes.
                        let file_ts = file_name
                            .strip_suffix(".tar.zst")
                            .and_then(|stem| stem.rsplit('-').next())
                            .and_then(|n| n.parse::<u128>().ok())
                            .unwrap_or(0);

                        // Primary ordering key.
                        let mtime_ns = modified
                            .duration_since(UNIX_EPOCH)
                            .map(|d| d.as_nanos())
                            .unwrap_or(0);

                        snapshots.push((path, file_ts, mtime_ns));
                    }
                }
            }
        }

        snapshots.sort_by(|a, b| {
            a.2.cmp(&b.2)
                .then(a.1.cmp(&b.1))
                .then_with(|| a.0.cmp(&b.0))
        });
        snapshots
            .last()
            .map(|(p, _, _)| p.clone())
            .ok_or_else(|| "no snapshot found for session".to_string())
    }

    /// Extracts all regular files from a zstd-compressed tar archive.
    fn extract_files_from_snapshot(compressed: &[u8]) -> Result<HashMap<String, Vec<u8>>, String> {
        let decoder = zstd::stream::read::Decoder::new(Cursor::new(compressed))
            .map_err(|e| format!("zstd init: {}", e))?;

        // Enforce 256MB limit on the decompressed stream
        let mut limited = decoder.take(256 * 1024 * 1024);

        let mut archive = tar::Archive::new(&mut limited);
        let mut files = HashMap::new();

        for entry in archive.entries().map_err(|e| e.to_string())? {
            let mut entry = entry.map_err(|e| e.to_string())?;
            let path_str = entry
                .path()
                .map_err(|e| e.to_string())?
                .to_string_lossy()
                .into_owned();
            if !entry.header().entry_type().is_file() {
                continue;
            }

            let mut buf = Vec::new();
            entry.read_to_end(&mut buf).map_err(|e| e.to_string())?;
            files.insert(path_str, buf);
        }

        Ok(files)
    }

    /// COW clone: suspend source -> rewrite snapshot with new UUID -> resume as new session.
    /// Source remains Suspended. Clone is fully independent with its own HMAC key.
    pub async fn clone_session(&self, source_id: Uuid) -> Result<Uuid, String> {
        let span = tracing::info_span!(
            "session.clone",
            source_id = %source_id,
            clone_id = tracing::field::Empty,
            elapsed_ms = tracing::field::Empty
        );
        async move {
            let start = Instant::now();

            // Suspend source — serializes all state to a snapshot on disk
            let snapshot_path_str = self.suspend(source_id).await?;
            let snapshot_path = PathBuf::from(&snapshot_path_str);

            let new_id = Uuid::new_v4();
            let new_id_str = new_id.to_string();

            // Create the clone's session directory with restricted perms
            let storage = self.storage.clone();
            let sid = new_id_str.clone();
            let new_session_dir =
                tokio::task::spawn_blocking(move || storage.create_session_dir(&sid))
                    .await
                    .map_err(|e| e.to_string())?
                    .map_err(|e| e.to_string())?;

            // Read source snapshot and rewrite the session_id + re-sign HMAC
            let original_bytes = tokio::fs::read(&snapshot_path)
                .await
                .map_err(|e| e.to_string())?;

            let sid2 = new_id_str.clone();
            let new_bytes = tokio::task::spawn_blocking(move || {
                Self::rewrite_snapshot_session_id(&original_bytes, &sid2)
            })
            .await
            .map_err(|e| e.to_string())??;

            // Write rewritten snapshot to clone's directory
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_millis();
            let new_snapshot_path =
                new_session_dir.join(format!("snapshot-{}-{}.tar.zst", new_id_str, timestamp));
            tokio::fs::write(&new_snapshot_path, &new_bytes)
                .await
                .map_err(|e| e.to_string())?;

            // Register as a new session mirroring the source's engine/budget/persona
            let source_session = self.broker.get(source_id).map_err(|e| e.to_string())?;
            let new_session = phantom_session::Session::with_uuid(
                new_id,
                source_session.engine,
                source_session.budget.clone(),
                source_session.persona_id.clone(),
            );
            self.broker.register(new_session);

            // Resume the clone — rehydrates cookies, localStorage, IndexedDB from the rewritten snapshot
            self.resume(new_id).await?;

            let elapsed = start.elapsed();
            if elapsed.as_millis() >= 200 {
                tracing::warn!(
                    "clone {} -> {} elapsed: {}ms (minimum: < 200ms)",
                    source_id,
                    new_id,
                    elapsed.as_millis()
                );
            }
            tracing::info!(
                "clone {} -> {} in {}ms",
                source_id,
                new_id,
                elapsed.as_millis()
            );

            tracing::Span::current().record("clone_id", new_id_str.as_str());
            tracing::Span::current().record("elapsed_ms", elapsed.as_millis() as u64);

            Ok(new_id)
        }
        .instrument(span)
        .await
    }

    /// Decompresses a snapshot, extracts all files, rebuilds with a new session_id.
    /// The HMAC is re-signed with the clone's derived key.
    fn rewrite_snapshot_session_id(
        compressed: &[u8],
        new_session_id: &str,
    ) -> Result<Vec<u8>, String> {
        let decoder = zstd::stream::read::Decoder::new(Cursor::new(compressed))
            .map_err(|e| format!("zstd init: {}", e))?;
        // Enforce 256MB limit
        let mut limited = decoder.take(256 * 1024 * 1024);

        // Extract every tar entry into a flat list
        let mut files: Vec<(String, Vec<u8>)> = Vec::new();
        let mut archive = tar::Archive::new(&mut limited);
        for entry in archive.entries().map_err(|e| e.to_string())? {
            let mut entry = entry.map_err(|e| e.to_string())?;
            let name = entry
                .path()
                .map_err(|e| e.to_string())?
                .to_string_lossy()
                .into_owned();

            // Skip the old manifest — build_snapshot generates a fresh one
            if name == "manifest.json" {
                continue;
            }

            let mut buf = Vec::new();
            entry.read_to_end(&mut buf).map_err(|e| e.to_string())?;
            files.push((name, buf));
        }

        // Partition extracted files into SnapshotData fields
        let mut cookies_json = Vec::new();
        let mut local_storage = HashMap::new();
        let mut indexeddb = HashMap::new();
        let mut cache_blobs = HashMap::new();
        let mut cache_meta = None;

        for (name, bytes) in files {
            if name == "cookies.bin" {
                cookies_json = bytes;
            } else if let Some(rest) = name.strip_prefix("localstorage/") {
                if let Some(hash) = rest.strip_suffix(".json") {
                    local_storage.insert(hash.to_string(), bytes);
                }
            } else if let Some(rest) = name.strip_prefix("indexeddb/") {
                if let Some(hash) = rest.strip_suffix(".sqlite") {
                    indexeddb.insert(hash.to_string(), bytes);
                }
            } else if let Some(blob_key) = name.strip_prefix("blobs/") {
                cache_blobs.insert(blob_key.to_string(), bytes);
            } else if name == "cache_meta.sled" {
                cache_meta = Some(bytes);
            }
        }

        let data = phantom_storage::snapshot::SnapshotData {
            session_id: new_session_id.to_string(),
            cookies_json,
            local_storage,
            indexeddb,
            cache_blobs,
            cache_meta,
        };

        phantom_storage::snapshot::build_snapshot(&data)
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
                            for (k, v) in db.iter().flatten() {
                                map.insert(
                                    String::from_utf8_lossy(&k).into_owned(),
                                    String::from_utf8_lossy(&v).into_owned(),
                                );
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
