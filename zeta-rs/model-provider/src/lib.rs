//! Model-provider adapters.

use zeta_core::{AgentModel, CoreError};

pub struct EchoModel;
impl AgentModel for EchoModel {
    fn respond(&self, prompt: &str) -> Result<String, CoreError> {
        Ok(format!("Zeta: {prompt}"))
    }
}
