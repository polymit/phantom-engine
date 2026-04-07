use std::sync::Arc;

use parking_lot::Mutex;
use phantom_anti_detect::{Persona, PersonaPool};
use phantom_js::tier1::pool::Tier1Pool;
use phantom_js::tier2::pool::Tier2Pool;
use phantom_net::SmartNetworkClient;
use phantom_session::SessionBroker;
use std::sync::Once;
use tokio::sync::OnceCell;

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
    TEST_ADAPTER.get_or_init(|| async {
        init_v8();
        // ZERO pre-warming for tests to avoid V8 isolate drop order panics across
        // multiple tests. Isolates will be created on-demand and dropped cleanly
        // within each test's lifecycle.
        let adapter = EngineAdapter::new(5, 0, 5, 0).await;
        Box::leak(Box::new(adapter)) as &'static EngineAdapter
    }).await
}

/// EngineAdapter is the single shared state type for the MCP server.
/// It owns all subsystems. Clone is cheap — all fields are Arc<T>.
#[derive(Clone)]
pub struct EngineAdapter {
    /// HTTP client built from the first persona in the pool.
    pub network:  Arc<SmartNetworkClient>,
    /// Session lifecycle manager.
    pub broker:   Arc<SessionBroker>,
    /// Pre-warmed QuickJS session pool (Tier 1).
    pub tier1:    Arc<Tier1Pool>,
    /// Pre-warmed V8 session pool (Tier 2).
    pub tier2:    Arc<Tier2Pool>,
    /// Persona pool for fingerprint rotation across sessions.
    pub personas: Arc<Mutex<PersonaPool>>,
}

impl EngineAdapter {
    /// Construct a new EngineAdapter and pre-warm both JS pools.
    ///
    /// Must be called after `v8::Platform` is initialised and after the
    /// Tokio runtime is started. `Tier1Pool::new()` is async; `Tier2Pool::new()`
    /// is synchronous.
    pub async fn new(
        t1_max: usize, t1_pre: usize,
        t2_max: usize, t2_pre: usize
    ) -> Self {
        let mut persona_pool = PersonaPool::default_pool();
        let first_persona = persona_pool.next_persona();
        let network = SmartNetworkClient::with_persona(&first_persona);

        let broker = SessionBroker::new();

        // Tier1Pool::new() already wraps in Arc internally.
        let tier1 = Tier1Pool::new(t1_max, t1_pre).await;

        // Tier2Pool::new() returns Self — we wrap it ourselves.
        let tier2 = Arc::new(Tier2Pool::new(t2_max, t2_pre));

        Self {
            network:  Arc::new(network),
            broker:   Arc::new(broker),
            tier1,
            tier2,
            personas: Arc::new(Mutex::new(persona_pool)),
        }
    }

    /// Rotate to the next persona and return it.
    /// Each new session should call this to vary the fingerprint.
    pub fn next_persona(&self) -> Persona {
        self.personas.lock().next_persona()
    }
}
