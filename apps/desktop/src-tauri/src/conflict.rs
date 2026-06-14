use std::sync::{Arc, Condvar, Mutex};

use once_cell::sync::Lazy;

use crate::domain::{ConflictAction, ResolveConflictInput};

#[derive(Debug, Clone)]
pub struct ConflictDecision {
    pub slug: String,
    pub action: ConflictAction,
    pub apply_to_all: bool,
    pub overwrite_confirmation_slug: Option<String>,
}

#[derive(Default)]
struct ConflictState {
    pub active_operation_id: Option<String>,
    pub cancel_requested: bool,
    pub apply_to_all_decision: Option<ConflictDecision>,
    pub pending_slug: Option<String>,
    pub pending_decision: Option<ConflictDecision>,
}

#[derive(Default)]
pub struct ConflictCoordinator {
    inner: Mutex<ConflictState>,
    cond: Condvar,
}

impl ConflictCoordinator {
    pub fn start_operation(&self, operation_id: &str) -> bool {
        let mut state = self.inner.lock().expect("lock");
        if state.active_operation_id.is_some() {
            return false;
        }
        state.active_operation_id = Some(operation_id.to_string());
        state.cancel_requested = false;
        state.apply_to_all_decision = None;
        state.pending_slug = None;
        state.pending_decision = None;
        true
    }

    pub fn finish_operation(&self, operation_id: &str) {
        let mut state = self.inner.lock().expect("lock");
        if state.active_operation_id.as_deref() == Some(operation_id) {
            state.active_operation_id = None;
            state.cancel_requested = false;
            state.apply_to_all_decision = None;
            state.pending_slug = None;
            state.pending_decision = None;
        }
        self.cond.notify_all();
    }

    pub fn request_cancel(&self, operation_id: &str) -> bool {
        let mut state = self.inner.lock().expect("lock");
        if state.active_operation_id.as_deref() != Some(operation_id) {
            return false;
        }
        state.cancel_requested = true;
        self.cond.notify_all();
        true
    }

    pub fn is_cancel_requested(&self, operation_id: &str) -> bool {
        let state = self.inner.lock().expect("lock");
        state.active_operation_id.as_deref() == Some(operation_id) && state.cancel_requested
    }

    pub fn wait_for_conflict_decision(&self, operation_id: &str, slug: &str) -> ConflictDecision {
        let mut state = self.inner.lock().expect("lock");
        if let Some(decision) = &state.apply_to_all_decision {
            return decision.clone();
        }

        state.pending_slug = Some(slug.to_string());
        state.pending_decision = None;
        while state.pending_decision.is_none() {
            state = self.cond.wait(state).expect("wait");
            if state.active_operation_id.as_deref() != Some(operation_id) {
                return ConflictDecision {
                    slug: slug.to_string(),
                    action: ConflictAction::Skip,
                    apply_to_all: false,
                    overwrite_confirmation_slug: None,
                };
            }
        }
        let decision = state.pending_decision.take().expect("pending_decision");
        state.pending_slug = None;
        if decision.apply_to_all {
            state.apply_to_all_decision = Some(decision.clone());
        }
        decision
    }

    pub fn resolve_conflict(&self, input: ResolveConflictInput) -> bool {
        let mut state = self.inner.lock().expect("lock");
        if state.active_operation_id.as_deref() != Some(input.operation_id.as_str()) {
            return false;
        }
        if state.pending_slug.as_deref() != Some(input.slug.as_str()) {
            return false;
        }
        if matches!(input.action, ConflictAction::Overwrite)
            && input.overwrite_confirmation_slug.as_deref() != Some(input.slug.as_str())
        {
            return false;
        }
        state.pending_decision = Some(ConflictDecision {
            slug: input.slug,
            action: input.action,
            apply_to_all: input.apply_to_all,
            overwrite_confirmation_slug: input.overwrite_confirmation_slug,
        });
        self.cond.notify_all();
        true
    }
}

pub static CONFLICT_COORDINATOR: Lazy<Arc<ConflictCoordinator>> =
    Lazy::new(|| Arc::new(ConflictCoordinator::default()));

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    use super::ConflictCoordinator;
    use crate::domain::{ConflictAction, ResolveConflictInput};

    #[test]
    fn supports_cancel_flag_for_active_operation() {
        let coordinator = ConflictCoordinator::default();
        assert!(coordinator.start_operation("op-1"));
        assert!(coordinator.request_cancel("op-1"));
        assert!(coordinator.is_cancel_requested("op-1"));
        coordinator.finish_operation("op-1");
    }

    #[test]
    fn waits_and_resolves_conflict_with_apply_to_all() {
        let coordinator = Arc::new(ConflictCoordinator::default());
        assert!(coordinator.start_operation("op-2"));
        let wait_coord = Arc::clone(&coordinator);
        let handle = thread::spawn(move || wait_coord.wait_for_conflict_decision("op-2", "dup"));

        thread::sleep(Duration::from_millis(50));
        let accepted = coordinator.resolve_conflict(ResolveConflictInput {
            operation_id: "op-2".to_string(),
            slug: "dup".to_string(),
            action: ConflictAction::Rename,
            apply_to_all: true,
            overwrite_confirmation_slug: None,
        });
        assert!(accepted);

        let decision = handle.join().expect("join");
        assert!(matches!(decision.action, ConflictAction::Rename));
        assert!(decision.apply_to_all);
        coordinator.finish_operation("op-2");
    }

    #[test]
    fn rejects_invalid_overwrite_confirmation() {
        let coordinator = Arc::new(ConflictCoordinator::default());
        assert!(coordinator.start_operation("op-3"));

        let wait_coord = Arc::clone(&coordinator);
        let _handle = thread::spawn(move || wait_coord.wait_for_conflict_decision("op-3", "dup"));
        thread::sleep(Duration::from_millis(50));

        let accepted = coordinator.resolve_conflict(ResolveConflictInput {
            operation_id: "op-3".to_string(),
            slug: "dup".to_string(),
            action: ConflictAction::Overwrite,
            apply_to_all: false,
            overwrite_confirmation_slug: Some("wrong".to_string()),
        });
        assert!(!accepted);
        coordinator.finish_operation("op-3");
    }
}
