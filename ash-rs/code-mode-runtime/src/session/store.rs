use super::RuntimeError;
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

/// Process-local values shared by Code Mode cells in one owning Thread Session.
#[derive(Clone, Default)]
pub struct CodeModeStore {
    values: Arc<Mutex<BTreeMap<String, Value>>>,
}

impl CodeModeStore {
    /// Creates an empty session store.
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_values(values: BTreeMap<String, Value>) -> Self {
        Self {
            values: Arc::new(Mutex::new(values)),
        }
    }

    pub fn snapshot(&self) -> Result<BTreeMap<String, Value>, RuntimeError> {
        self.values
            .lock()
            .map_err(|_| RuntimeError::Runtime("Code Mode values were poisoned".into()))
            .map(|values| values.clone())
    }

    pub fn extend(&self, writes: BTreeMap<String, Value>) -> Result<(), RuntimeError> {
        self.values
            .lock()
            .map_err(|_| RuntimeError::Runtime("Code Mode values were poisoned".into()))?
            .extend(writes);
        Ok(())
    }
}
