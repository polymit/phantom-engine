use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SessionState {
    Running,
    Suspended,
    Cloned,
    Destroyed,
    Idle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EngineKind {
    V8,
    QuickJs,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceBudget {
    pub max_heap_bytes: usize,
    pub max_cpu_ms_per_sec: u64,
    pub max_network_bytes: usize,
}

impl Default for ResourceBudget {
    fn default() -> Self {
        Self {
            max_heap_bytes: 512 * 1024 * 1024,
            max_cpu_ms_per_sec: 1000,
            max_network_bytes: 64 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Session {
    pub id: Uuid,
    pub created_at: Instant,
    pub last_access: Instant,
    pub state: SessionState,
    pub snapshot_id: Option<String>,
    pub budget: ResourceBudget,
    pub engine: EngineKind,
    pub persona_id: String,
    pub heap_bytes_used: usize,
    pub cpu_ms_used: u64,
    pub network_bytes_used: usize,
}

impl Session {
    pub fn new(engine: EngineKind, budget: ResourceBudget, persona_id: impl Into<String>) -> Self {
        let now = Instant::now();
        Self {
            id: Uuid::new_v4(),
            created_at: now,
            last_access: now,
            state: SessionState::Idle,
            snapshot_id: None,
            budget,
            engine,
            persona_id: persona_id.into(),
            heap_bytes_used: 0,
            cpu_ms_used: 0,
            network_bytes_used: 0,
        }
    }

    /// Create a session with a pre-chosen UUID — used for COW cloning.
    /// Normal sessions use `new()` which calls `Uuid::new_v4()` internally.
    pub fn with_uuid(
        id: Uuid,
        engine: EngineKind,
        budget: ResourceBudget,
        persona_id: impl Into<String>,
    ) -> Self {
        let now = Instant::now();
        Self {
            id,
            created_at: now,
            last_access: now,
            state: SessionState::Idle,
            snapshot_id: None,
            budget,
            engine,
            persona_id: persona_id.into(),
            heap_bytes_used: 0,
            cpu_ms_used: 0,
            network_bytes_used: 0,
        }
    }

    pub fn touch(&mut self) {
        self.last_access = Instant::now();
    }

    pub fn set_state(&mut self, state: SessionState) {
        self.state = state;
        self.touch();
    }

    pub fn memory_used(&self) -> usize {
        self.heap_bytes_used
    }

    pub fn cpu_used(&self) -> u64 {
        self.cpu_ms_used
    }

    pub fn network_used(&self) -> usize {
        self.network_bytes_used
    }

    pub fn record_usage(&mut self, heap_bytes: usize, cpu_ms: u64, network_bytes: usize) {
        self.heap_bytes_used = self.heap_bytes_used.saturating_add(heap_bytes);
        self.cpu_ms_used = self.cpu_ms_used.saturating_add(cpu_ms);
        self.network_bytes_used = self.network_bytes_used.saturating_add(network_bytes);
        self.touch();
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SessionError {
    #[error("session not found: {0}")]
    NotFound(Uuid),
    #[error("budget exceeded: {resource} {used}/{limit}")]
    BudgetExceeded {
        resource: String,
        used: u64,
        limit: u64,
    },
}

/// Minimal in-memory session registry used by higher layers.
#[derive(Debug, Default)]
pub struct SessionBroker {
    sessions: Mutex<HashMap<Uuid, Session>>,
}

impl SessionBroker {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
        }
    }

    pub fn register(&self, session: Session) -> Uuid {
        let id = session.id;
        self.lock_sessions().insert(id, session);
        id
    }

    pub fn create(
        &self,
        engine: EngineKind,
        budget: ResourceBudget,
        persona_id: impl Into<String>,
    ) -> Uuid {
        self.register(Session::new(engine, budget, persona_id))
    }

    pub fn create_session(
        &self,
        engine: EngineKind,
        budget: ResourceBudget,
        persona_id: impl Into<String>,
    ) -> Result<Uuid, SessionError> {
        let session = Session::new(engine, budget, persona_id);
        Self::check_budget_not_exceeded(&session)?;
        Ok(self.register(session))
    }

    pub fn get(&self, id: Uuid) -> Result<Session, SessionError> {
        self.lock_sessions()
            .get(&id)
            .cloned()
            .ok_or(SessionError::NotFound(id))
    }

    pub fn set_state(&self, id: Uuid, state: SessionState) -> Result<(), SessionError> {
        let mut guard = self.lock_sessions();
        let session = guard.get_mut(&id).ok_or(SessionError::NotFound(id))?;
        session.set_state(state);
        Ok(())
    }

    pub fn remove(&self, id: Uuid) -> Result<Session, SessionError> {
        self.lock_sessions()
            .remove(&id)
            .ok_or(SessionError::NotFound(id))
    }

    pub fn len(&self) -> usize {
        self.lock_sessions().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn record_usage_and_check(
        &self,
        id: Uuid,
        heap_bytes: usize,
        cpu_ms: u64,
        network_bytes: usize,
    ) -> Result<(), SessionError> {
        let mut guard = self.lock_sessions();
        let session = guard.get_mut(&id).ok_or(SessionError::NotFound(id))?;
        session.record_usage(heap_bytes, cpu_ms, network_bytes);
        Self::check_budget_not_exceeded(session)
    }

    pub fn check_budget_not_exceeded(session: &Session) -> Result<(), SessionError> {
        if session.memory_used() > session.budget.max_heap_bytes {
            return Err(SessionError::BudgetExceeded {
                resource: "heap_bytes".to_string(),
                used: session.memory_used() as u64,
                limit: session.budget.max_heap_bytes as u64,
            });
        }
        if session.cpu_used() > session.budget.max_cpu_ms_per_sec {
            return Err(SessionError::BudgetExceeded {
                resource: "cpu_ms_per_sec".to_string(),
                used: session.cpu_used(),
                limit: session.budget.max_cpu_ms_per_sec,
            });
        }
        if session.network_used() > session.budget.max_network_bytes {
            return Err(SessionError::BudgetExceeded {
                resource: "network_bytes".to_string(),
                used: session.network_used() as u64,
                limit: session.budget.max_network_bytes as u64,
            });
        }
        Ok(())
    }

    fn lock_sessions(&self) -> std::sync::MutexGuard<'_, HashMap<Uuid, Session>> {
        match self.sessions.lock() {
            Ok(guard) => guard,
            // Keep operating with the inner value so a prior panic does not take down the broker.
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::{EngineKind, ResourceBudget, SessionBroker, SessionState};

    #[test]
    fn broker_tracks_session_lifecycle() {
        let broker = SessionBroker::new();
        let id = broker.create(EngineKind::V8, ResourceBudget::default(), "persona_a");

        let session = broker.get(id).unwrap();
        assert_eq!(session.state, SessionState::Idle);

        broker.set_state(id, SessionState::Running).unwrap();
        let session = broker.get(id).unwrap();
        assert_eq!(session.state, SessionState::Running);

        let removed = broker.remove(id).unwrap();
        assert_eq!(removed.state, SessionState::Running);
        assert!(broker.is_empty());
    }
}
