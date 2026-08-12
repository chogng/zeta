use crate::context::ContextInput;
use crate::context::ContextPlanner;
use crate::context::ContextPreparation;
use crate::context::ContextPreparationError;

/// Coordinates context preparation for one loaded Thread without owning canonical history.
///
/// The first implementation intentionally holds no plan cache, reference baseline, or token
/// estimate. Dropping it and preparing again from the same durable facts must produce an
/// equivalent plan.
#[derive(Default)]
pub(crate) struct ContextManager {
    observed_thread_sequence: u64,
}

impl ContextManager {
    pub(crate) fn prepare(
        &mut self,
        input: &ContextInput,
    ) -> Result<ContextPreparation, ContextPreparationError> {
        if input.source_thread_sequence() < self.observed_thread_sequence {
            return Err(ContextPreparationError::UnsupportedContextShape(format!(
                "context input sequence {} is older than observed Thread sequence {}",
                input.source_thread_sequence(),
                self.observed_thread_sequence
            )));
        }
        let preparation = ContextPlanner::prepare(input)?;
        self.observed_thread_sequence = input.source_thread_sequence();
        Ok(preparation)
    }
}

#[cfg(test)]
#[path = "context_manager_tests.rs"]
mod tests;
