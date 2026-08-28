use super::remaining_context_window;
use crate::features::status::RemainingContextWindow;
use zeta_protocol::ApprovalMode;
use zeta_protocol::ModelContextUsage;
use zeta_protocol::ModelContextUsageSource;
use zeta_protocol::ModelId;
use zeta_protocol::ModelRef;
use zeta_protocol::ProviderId;
use zeta_protocol::SessionId;
use zeta_protocol::Thread;
use zeta_protocol::ThreadId;
use zeta_protocol::ThreadStatus;
use zeta_protocol::ToolMode;
use zeta_protocol::Turn;
use zeta_protocol::TurnId;
use zeta_protocol::TurnStatus;

#[test]
fn remaining_context_uses_only_the_latest_matching_turn_window() {
    let selected_model = model("gpt-zeta");
    let mut thread = thread(selected_model.clone());

    assert_eq!(
        remaining_context_window(Some(90_000), Some(&selected_model), &thread),
        RemainingContextWindow::Exact {
            remaining_tokens: 65_000,
            available_tokens: 90_000,
        }
    );

    thread.turns[0].context_usage = Some(ModelContextUsage {
        used_tokens: 30_000,
        source: ModelContextUsageSource::Estimated,
    });
    assert_eq!(
        remaining_context_window(Some(90_000), Some(&selected_model), &thread),
        RemainingContextWindow::Estimated {
            remaining_tokens: 60_000,
            available_tokens: 90_000,
        }
    );

    thread.turns[0].model = Some(model("another-model"));
    assert_eq!(
        remaining_context_window(Some(90_000), Some(&selected_model), &thread),
        RemainingContextWindow::Unknown
    );
}

fn model(name: &str) -> ModelRef {
    ModelRef::new(
        ProviderId::new("openai").unwrap(),
        ModelId::new(name).unwrap(),
    )
}

fn thread(model: ModelRef) -> Thread {
    Thread {
        session_id: SessionId::new("session-1").unwrap(),
        thread_id: ThreadId::new("thread-1").unwrap(),
        title: "test".into(),
        status: ThreadStatus::Active,
        sequence: 4,
        usage: zeta_protocol::ModelUsageSummary::default(),
        goal: None,
        turns: vec![Turn {
            turn_id: TurnId::new("turn-1").unwrap(),
            status: TurnStatus::Completed,
            model: Some(model),
            tool_profile: None,
            tool_mode: ToolMode::Direct,
            approval_mode: ApprovalMode::AskPermissions,
            usage: zeta_protocol::ModelUsageSummary::default(),
            context_usage: Some(ModelContextUsage {
                used_tokens: 25_000,
                source: ModelContextUsageSource::ProviderReported,
            }),
            items: Vec::new(),
            plan: None,
            pending_interaction: None,
            error: None,
        }],
    }
}
