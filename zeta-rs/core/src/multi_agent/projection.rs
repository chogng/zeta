use std::collections::BTreeMap;
use std::collections::BTreeSet;

use crate::SessionSnapshot;
use crate::ThreadSnapshot;
use zeta_protocol::AgentTreeExecutionStatus;
use zeta_protocol::AgentTreeNodeProjection;
use zeta_protocol::AgentTreeProjection;
use zeta_protocol::AgentTreeWaitingReason;
use zeta_protocol::DelegationResult;
use zeta_protocol::ModelUsageSummary;
use zeta_protocol::ThreadId;
use zeta_protocol::ThreadOrigin;
use zeta_protocol::TurnStatus;

/// Derives the complete Agent observability tree from one consistent Session/Thread read set.
pub fn project_agent_tree(
    session: &SessionSnapshot,
    threads: &[ThreadSnapshot],
) -> AgentTreeProjection {
    let by_id = threads
        .iter()
        .map(|thread| (thread.thread_id.clone(), thread))
        .collect::<BTreeMap<_, _>>();
    let memberships = session
        .threads
        .iter()
        .map(|thread| (thread.membership.thread_id.clone(), &thread.membership))
        .collect::<BTreeMap<_, _>>();
    let mut children = BTreeMap::<ThreadId, Vec<ThreadId>>::new();
    let mut roots = Vec::new();
    for membership in &session.threads {
        let thread_id = membership.membership.thread_id.clone();
        match parent_thread_id(&membership.membership.origin) {
            Some(parent) if parent != &thread_id && memberships.contains_key(parent) => {
                children.entry(parent.clone()).or_default().push(thread_id);
            }
            _ => roots.push(thread_id),
        }
    }
    let results = threads
        .iter()
        .flat_map(|thread| thread.received_delegation_results.values())
        .map(|result| (result.child_thread_id.clone(), result.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut projected = BTreeSet::new();
    let mut nodes = roots
        .iter()
        .filter_map(|thread_id| {
            build_node(
                thread_id,
                &by_id,
                &memberships,
                &children,
                &results,
                &mut projected,
            )
        })
        .collect::<Vec<_>>();
    for membership in &session.threads {
        if !projected.contains(&membership.membership.thread_id)
            && let Some(node) = build_node(
                &membership.membership.thread_id,
                &by_id,
                &memberships,
                &children,
                &results,
                &mut projected,
            )
        {
            nodes.push(node);
        }
    }
    AgentTreeProjection { roots: nodes }
}

fn build_node(
    thread_id: &ThreadId,
    threads: &BTreeMap<ThreadId, &ThreadSnapshot>,
    memberships: &BTreeMap<ThreadId, &zeta_protocol::SessionThread>,
    children: &BTreeMap<ThreadId, Vec<ThreadId>>,
    results: &BTreeMap<ThreadId, DelegationResult>,
    projected: &mut BTreeSet<ThreadId>,
) -> Option<AgentTreeNodeProjection> {
    let thread = threads.get(thread_id)?;
    let membership = memberships.get(thread_id)?;
    if !projected.insert(thread_id.clone()) {
        return None;
    }
    let turn = thread.turns.last();
    Some(AgentTreeNodeProjection {
        thread_id: thread_id.clone(),
        thread_sequence: thread.sequence,
        title: thread.title.clone(),
        origin: membership.origin.clone(),
        membership_status: membership.status,
        execution_status: turn
            .map(|turn| execution_status(turn.status))
            .unwrap_or(AgentTreeExecutionStatus::Idle),
        current_turn_id: turn.map(|turn| turn.turn_id.clone()),
        waiting_reason: turn.and_then(|turn| waiting_reason(turn.status)),
        usage: turn
            .map(|turn| turn.usage.clone())
            .unwrap_or_else(ModelUsageSummary::default),
        goal: thread.goal.clone(),
        role: thread
            .agent_context_seed
            .as_ref()
            .and_then(|seed| seed.role.definition.clone()),
        result: results.get(thread_id).cloned(),
        joins: thread.agent_joins.values().cloned().collect(),
        children: children
            .get(thread_id)
            .into_iter()
            .flatten()
            .filter_map(|child| {
                build_node(child, threads, memberships, children, results, projected)
            })
            .collect(),
    })
}

fn parent_thread_id(origin: &ThreadOrigin) -> Option<&ThreadId> {
    match origin {
        ThreadOrigin::Root => None,
        ThreadOrigin::Fork {
            parent_thread_id, ..
        }
        | ThreadOrigin::Rewind {
            parent_thread_id, ..
        }
        | ThreadOrigin::AgentSpawn {
            parent_thread_id, ..
        } => Some(parent_thread_id),
    }
}

fn execution_status(status: TurnStatus) -> AgentTreeExecutionStatus {
    match status {
        TurnStatus::Created => AgentTreeExecutionStatus::Queued,
        TurnStatus::Running | TurnStatus::Cancelling => AgentTreeExecutionStatus::Running,
        TurnStatus::WaitingForApproval
        | TurnStatus::WaitingForUserInput
        | TurnStatus::WaitingForCapability => AgentTreeExecutionStatus::Waiting,
        TurnStatus::Completed => AgentTreeExecutionStatus::Completed,
        TurnStatus::Failed => AgentTreeExecutionStatus::Failed,
        TurnStatus::Interrupted => AgentTreeExecutionStatus::Cancelled,
    }
}

fn waiting_reason(status: TurnStatus) -> Option<AgentTreeWaitingReason> {
    match status {
        TurnStatus::WaitingForApproval => Some(AgentTreeWaitingReason::Approval),
        TurnStatus::WaitingForUserInput => Some(AgentTreeWaitingReason::UserInput),
        TurnStatus::WaitingForCapability => Some(AgentTreeWaitingReason::Capability),
        TurnStatus::Created
        | TurnStatus::Running
        | TurnStatus::Cancelling
        | TurnStatus::Completed
        | TurnStatus::Failed
        | TurnStatus::Interrupted => None,
    }
}
