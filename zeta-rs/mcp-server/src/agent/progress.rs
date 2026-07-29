use zeta_protocol::{ThreadEvent, ThreadUpdate, ThreadUpdateEnvelope, TurnId, TurnInteraction};

pub(super) enum TurnUpdate {
    Progress(String),
    Interaction(TurnInteraction),
}

pub(super) fn project(update: &ThreadUpdateEnvelope, turn_id: &TurnId) -> Option<TurnUpdate> {
    match &update.update {
        ThreadUpdate::Committed { event } => committed(event, turn_id),
        ThreadUpdate::ItemStarted {
            turn_id: update_turn,
            ..
        } if update_turn == turn_id => Some(TurnUpdate::Progress("Agent item started".into())),
        ThreadUpdate::ItemDelta {
            turn_id: update_turn,
            delta,
            ..
        } if update_turn == turn_id => Some(TurnUpdate::Progress(
            match delta {
                zeta_protocol::ItemDelta::AgentMessage { .. } => "Agent response updated",
                zeta_protocol::ItemDelta::Reasoning { .. } => "Agent reasoning updated",
                zeta_protocol::ItemDelta::Plan { .. } => "Agent plan updated",
            }
            .into(),
        )),
        ThreadUpdate::PlanUpdated {
            turn_id: update_turn,
            ..
        } if update_turn == turn_id => Some(TurnUpdate::Progress("Agent plan updated".into())),
        _ => None,
    }
}

fn committed(event: &ThreadEvent, turn_id: &TurnId) -> Option<TurnUpdate> {
    let (event_turn, message) = match event {
        ThreadEvent::TurnAccepted { turn_id, .. } => (turn_id, "Turn accepted"),
        ThreadEvent::TurnStarted { turn_id, .. } => (turn_id, "Turn started"),
        ThreadEvent::ItemCompleted { turn_id, .. } => (turn_id, "Agent item completed"),
        ThreadEvent::InteractionRequested {
            turn_id: event_turn,
            interaction,
            ..
        } if event_turn == turn_id => return Some(TurnUpdate::Interaction(interaction.clone())),
        ThreadEvent::InteractionResolved { turn_id, .. } => (turn_id, "Interaction resolved"),
        ThreadEvent::ToolExecutionStarted { turn_id, .. } => (turn_id, "Tool execution started"),
        ThreadEvent::ToolExecutionEscalated { turn_id, .. } => {
            (turn_id, "Tool execution escalated")
        }
        ThreadEvent::InteractionCancelled { turn_id, .. } => (turn_id, "Interaction cancelled"),
        ThreadEvent::TurnCompleted { turn_id, .. } => (turn_id, "Turn completed"),
        ThreadEvent::TurnFailed { turn_id, .. } => (turn_id, "Turn failed"),
        ThreadEvent::TurnCancelling { turn_id, .. } => (turn_id, "Turn cancelling"),
        ThreadEvent::TurnInterrupted { turn_id, .. } => (turn_id, "Turn interrupted"),
        ThreadEvent::ThreadCreated { .. } | ThreadEvent::InteractionRequested { .. } => {
            return None;
        }
    };
    (event_turn == turn_id).then(|| TurnUpdate::Progress(message.into()))
}
