use super::PlanProgress;
use zeta_protocol::PlanStep;
use zeta_protocol::PlanStepStatus;
use zeta_protocol::PlanUpdate;

#[test]
fn replacing_a_plan_preserves_its_expansion_state() {
    let mut progress = PlanProgress::new(plan(PlanStepStatus::InProgress));
    progress.toggle_expanded();

    progress.replace(plan(PlanStepStatus::Completed));

    assert!(progress.view().expanded);
    assert!(progress.is_complete());
}

#[test]
fn an_empty_plan_is_not_treated_as_complete() {
    let progress = PlanProgress::new(PlanUpdate {
        explanation: None,
        steps: Vec::new(),
    });

    assert!(!progress.is_complete());
}

fn plan(status: PlanStepStatus) -> PlanUpdate {
    PlanUpdate {
        explanation: None,
        steps: vec![PlanStep {
            step: "Implement".into(),
            status,
        }],
    }
}
