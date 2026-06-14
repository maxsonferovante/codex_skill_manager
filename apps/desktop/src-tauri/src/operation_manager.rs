use std::sync::{Arc, Mutex};

use once_cell::sync::Lazy;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationModeKind {
    SkillsMove,
    PdfConversion,
    SkillCreation,
}

#[derive(Debug, Clone)]
pub struct ActiveOperation {
    pub operation_id: String,
    pub mode: OperationModeKind,
}

#[derive(Default)]
pub struct OperationManager {
    active: Mutex<Option<ActiveOperation>>,
}

impl OperationManager {
    pub fn try_start(&self, operation_id: &str, mode: OperationModeKind) -> bool {
        let mut active = self.active.lock().expect("lock");
        if active.is_some() {
            return false;
        }
        *active = Some(ActiveOperation {
            operation_id: operation_id.to_string(),
            mode,
        });
        true
    }

    pub fn finish_if_matches(&self, operation_id: &str) {
        let mut active = self.active.lock().expect("lock");
        if active
            .as_ref()
            .map(|item| item.operation_id.as_str())
            == Some(operation_id)
        {
            *active = None;
        }
    }

    pub fn get_active(&self) -> Option<ActiveOperation> {
        self.active.lock().expect("lock").clone()
    }
}

pub static OPERATION_MANAGER: Lazy<Arc<OperationManager>> =
    Lazy::new(|| Arc::new(OperationManager::default()));
