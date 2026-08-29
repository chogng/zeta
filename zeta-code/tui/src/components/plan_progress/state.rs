use zeta_protocol::PlanStepStatus;
use zeta_protocol::PlanUpdate;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PlanProgress {
    plan: PlanUpdate,
    expanded: bool,
}

impl PlanProgress {
    pub(crate) fn new(plan: PlanUpdate) -> Self {
        Self {
            plan,
            expanded: false,
        }
    }

    pub(crate) fn replace(&mut self, plan: PlanUpdate) {
        self.plan = plan;
    }

    pub(crate) fn toggle_expanded(&mut self) {
        self.expanded = !self.expanded;
    }

    pub(crate) fn is_complete(&self) -> bool {
        !self.plan.steps.is_empty()
            && self
                .plan
                .steps
                .iter()
                .all(|step| step.status == PlanStepStatus::Completed)
    }

    pub(crate) fn view(&self) -> PlanProgressView<'_> {
        PlanProgressView {
            plan: &self.plan,
            expanded: self.expanded,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct PlanProgressView<'a> {
    pub(crate) plan: &'a PlanUpdate,
    pub(crate) expanded: bool,
}

impl PlanProgressView<'_> {
    pub(crate) fn completed_steps(self) -> usize {
        self.plan
            .steps
            .iter()
            .filter(|step| step.status == PlanStepStatus::Completed)
            .count()
    }
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
