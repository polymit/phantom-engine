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
            max_cpu_ms_per_sec: 250,
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
        }
    }

    pub fn touch(&mut self) {
        self.last_access = Instant::now();
    }

    pub fn set_state(&mut self, state: SessionState) {
        self.state = state;
        self.touch();
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SessionError {
    #[error("session not found: {0}")]
    NotFound(Uuid),
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
        self.sessions.lock().unwrap().insert(id, session);
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

    pub fn get(&self, id: Uuid) -> Result<Session, SessionError> {
        self.sessions
            .lock()
            .unwrap()
            .get(&id)
            .cloned()
            .ok_or(SessionError::NotFound(id))
    }

    pub fn set_state(&self, id: Uuid, state: SessionState) -> Result<(), SessionError> {
        let mut guard = self.sessions.lock().unwrap();
        let session = guard.get_mut(&id).ok_or(SessionError::NotFound(id))?;
        session.set_state(state);
        Ok(())
    }

    pub fn remove(&self, id: Uuid) -> Result<Session, SessionError> {
        self.sessions
            .lock()
            .unwrap()
            .remove(&id)
            .ok_or(SessionError::NotFound(id))
    }

    pub fn len(&self) -> usize {
        self.sessions.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
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
