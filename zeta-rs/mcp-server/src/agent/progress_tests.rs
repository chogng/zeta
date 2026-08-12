use super::committed;
use zeta_protocol::ContextCheckpoint;
use zeta_protocol::ContextCheckpointId;
use zeta_protocol::ContextCheckpointVerification;
use zeta_protocol::ContextSourceDigest;
use zeta_protocol::ContextSourceRange;
use zeta_protocol::ThreadEvent;
use zeta_protocol::ThreadId;
use zeta_protocol::TurnId;

#[test]
fn context_checkpoint_is_not_projected_as_turn_progress() {
    let thread_id = ThreadId::new("thread-1").unwrap();
    let event = ThreadEvent::ContextCheckpointCommitted {
        thread_id: thread_id.clone(),
        checkpoint: ContextCheckpoint {
            checkpoint_id: ContextCheckpointId::new("checkpoint-1").unwrap(),
            source_thread_id: thread_id,
            covered: ContextSourceRange {
                start_sequence: 1,
                end_sequence: 3,
            },
            referenced_items: Vec::new(),
            source_digest: ContextSourceDigest::new(format!("sha256:{}", "a".repeat(64))).unwrap(),
            summary: "Earlier context".into(),
            schema_revision: "context-checkpoint-v1".into(),
            prompt_revision: "context-compaction-v1".into(),
            context_policy_revision: "context-policy-v1".into(),
            generator_model: None,
            created_at_unix_ms: 1,
            verification: ContextCheckpointVerification::Verified,
        },
    };

    assert!(committed(&event, &TurnId::new("turn-1").unwrap()).is_none());
}
