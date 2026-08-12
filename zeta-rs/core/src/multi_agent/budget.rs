use crate::CoreError;
use crate::SessionSnapshot;
use std::collections::BTreeSet;
use zeta_protocol::SessionThreadStatus;
use zeta_protocol::ThreadId;
use zeta_protocol::ThreadOrigin;

/// Structural resource ceilings applied before a child Agent Thread is reserved.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AgentTreeLimits {
    max_depth: u32,
    max_live_children: u32,
    max_total_descendants: u32,
}

impl AgentTreeLimits {
    /// Creates non-zero structural limits for one Agent tree.
    pub fn new(
        max_depth: u32,
        max_live_children: u32,
        max_total_descendants: u32,
    ) -> Result<Self, CoreError> {
        if max_depth == 0 || max_live_children == 0 || max_total_descendants == 0 {
            return Err(CoreError::InvalidInput(
                "Agent tree limits must be non-zero".into(),
            ));
        }
        Ok(Self {
            max_depth,
            max_live_children,
            max_total_descendants,
        })
    }
}

impl Default for AgentTreeLimits {
    fn default() -> Self {
        Self {
            max_depth: 4,
            max_live_children: 4,
            max_total_descendants: 16,
        }
    }
}

pub(super) fn validate_spawn_capacity(
    session: &SessionSnapshot,
    parent_thread_id: &ThreadId,
    limits: AgentTreeLimits,
) -> Result<(), CoreError> {
    let parent_depth = agent_depth(session, parent_thread_id)?;
    if parent_depth.saturating_add(1) > limits.max_depth {
        return Err(CoreError::InvalidInput(
            "Agent tree maximum depth has been reached".into(),
        ));
    }
    let live_children = session
        .threads
        .iter()
        .filter(|thread| {
            matches!(
                &thread.membership.origin,
                ThreadOrigin::AgentSpawn {
                    parent_thread_id: parent,
                    ..
                } if parent == parent_thread_id
            ) && matches!(
                thread.membership.status,
                SessionThreadStatus::Creating | SessionThreadStatus::Active
            )
        })
        .count() as u32;
    if live_children >= limits.max_live_children {
        return Err(CoreError::InvalidInput(
            "Agent parent maximum live-child count has been reached".into(),
        ));
    }

    let root = agent_root(session, parent_thread_id)?;
    let descendants = session
        .threads
        .iter()
        .filter(|thread| {
            matches!(thread.membership.origin, ThreadOrigin::AgentSpawn { .. })
                && agent_root(session, &thread.membership.thread_id)
                    .is_ok_and(|candidate| candidate == root)
        })
        .count() as u32;
    if descendants >= limits.max_total_descendants {
        return Err(CoreError::InvalidInput(
            "Agent tree maximum descendant count has been reached".into(),
        ));
    }
    Ok(())
}

fn agent_depth(session: &SessionSnapshot, thread_id: &ThreadId) -> Result<u32, CoreError> {
    let root = agent_root_and_depth(session, thread_id)?;
    Ok(root.1)
}

fn agent_root(session: &SessionSnapshot, thread_id: &ThreadId) -> Result<ThreadId, CoreError> {
    Ok(agent_root_and_depth(session, thread_id)?.0)
}

fn agent_root_and_depth(
    session: &SessionSnapshot,
    thread_id: &ThreadId,
) -> Result<(ThreadId, u32), CoreError> {
    let mut current = thread_id.clone();
    let mut depth = 0_u32;
    let mut seen = BTreeSet::new();
    loop {
        if !seen.insert(current.clone()) {
            return Err(CoreError::Journal(
                "Agent Thread lineage contains a cycle".into(),
            ));
        }
        let thread = session
            .threads
            .iter()
            .find(|thread| thread.membership.thread_id == current)
            .ok_or_else(|| CoreError::NotFound(current.to_string()))?;
        match &thread.membership.origin {
            ThreadOrigin::AgentSpawn {
                parent_thread_id, ..
            } => {
                current = parent_thread_id.clone();
                depth = depth.saturating_add(1);
            }
            ThreadOrigin::Root | ThreadOrigin::Fork { .. } | ThreadOrigin::Rewind { .. } => {
                return Ok((current, depth));
            }
        }
    }
}
