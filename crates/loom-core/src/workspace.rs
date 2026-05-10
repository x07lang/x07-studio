use indexmap::IndexMap;
use uuid::Uuid;

use loom_types::artifacts::TaskType;
use loom_types::ops::SessionEvent;
use loom_types::session::SessionSnapshot;

use crate::reducer::{apply_event, TransitionError};

#[derive(Debug, Clone)]
pub struct WorkspaceModel {
    pub root: String,
    pub sessions: IndexMap<Uuid, SessionSnapshot>,
    pub selected_session: Option<Uuid>,
}

impl WorkspaceModel {
    pub fn new(root: impl Into<String>) -> Self {
        Self {
            root: root.into(),
            sessions: IndexMap::new(),
            selected_session: None,
        }
    }

    pub fn from_sessions(root: impl Into<String>, sessions: Vec<SessionSnapshot>) -> Self {
        let root = root.into();
        let mut model = Self::new(root);
        for session in sessions {
            let id = session.session_id;
            model.sessions.insert(id, session);
        }
        model.selected_session = model.sessions.first().map(|(id, _)| *id);
        model
    }

    pub fn create_session(&mut self, title: impl Into<String>, task_type: TaskType) -> Uuid {
        let session_id = Uuid::new_v4();
        let session = SessionSnapshot::new(session_id, title, self.root.clone(), task_type);
        self.sessions.insert(session_id, session);
        self.selected_session = Some(session_id);
        session_id
    }

    pub fn load_session(&mut self, session: SessionSnapshot) {
        let session_id = session.session_id;
        self.sessions.insert(session_id, session);
        if self.selected_session.is_none() {
            self.selected_session = Some(session_id);
        }
    }

    pub fn get_session(&self, session_id: Uuid) -> Option<&SessionSnapshot> {
        self.sessions.get(&session_id)
    }

    pub fn get_session_mut(&mut self, session_id: Uuid) -> Option<&mut SessionSnapshot> {
        self.sessions.get_mut(&session_id)
    }

    pub fn selected_session(&self) -> Option<&SessionSnapshot> {
        self.selected_session.and_then(|id| self.sessions.get(&id))
    }

    pub fn session_list(&self) -> Vec<SessionSnapshot> {
        self.sessions.values().cloned().collect()
    }

    pub fn select_session(&mut self, session_id: Uuid) {
        if self.sessions.contains_key(&session_id) {
            self.selected_session = Some(session_id);
        }
    }

    pub fn dispatch(
        &mut self,
        session_id: Uuid,
        event: SessionEvent,
    ) -> Result<SessionSnapshot, TransitionError> {
        let session = self
            .get_session_mut(session_id)
            .ok_or(TransitionError::UnknownSession { session_id })?;
        apply_event(session, event)?;
        Ok(session.clone())
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use loom_types::artifacts::TaskType;
    use loom_types::ops::SessionEvent;
    use loom_types::session::SessionPhase;

    use crate::reducer::TransitionError;

    use super::WorkspaceModel;

    #[test]
    fn dispatch_reports_unknown_session_without_panicking() {
        let mut model = WorkspaceModel::new("/workspace");
        let session_id = Uuid::new_v4();

        let error = model
            .dispatch(session_id, SessionEvent::DraftSpec)
            .expect_err("unknown session should fail");

        assert!(matches!(
            error,
            TransitionError::UnknownSession { session_id: id } if id == session_id
        ));
    }

    #[test]
    fn create_session_selects_new_session() {
        let mut model = WorkspaceModel::new("/workspace");

        let first = model.create_session("first", TaskType::NewBehavior);
        let second = model.create_session("second", TaskType::BugFix);

        assert_eq!(
            model.selected_session().map(|session| session.session_id),
            Some(second)
        );
        assert_eq!(
            model.get_session(first).expect("first").phase,
            SessionPhase::IntentDrafting
        );
        assert_eq!(model.session_list().len(), 2);
    }
}
