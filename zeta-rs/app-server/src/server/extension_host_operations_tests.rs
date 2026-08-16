use super::cancellation_reason;
use super::failure_code;
use super::output_event_dto;
use zeta_app_server_protocol::protocol::extension_host::ExtensionHostCancellationReasonDto;
use zeta_app_server_protocol::protocol::extension_host::ExtensionHostFailureCodeDto;
use zeta_app_server_protocol::protocol::extension_host::ExtensionHostOutputOperationDto;
use zeta_editor_extension_host::CancelReason;
use zeta_editor_extension_host::ExtensionHostOutputEvent;
use zeta_editor_extension_host::HostEventContext;
use zeta_editor_extension_host::HostOutputOperation;
use zeta_editor_extension_host::HostOutputSeverity;
use zeta_editor_extension_host::SequencedExtensionHostOutputEvent;

#[test]
fn cancellation_reasons_are_projected_without_losing_authority_revocation() {
    assert_eq!(
        cancellation_reason(CancelReason::AuthorityRevoked),
        ExtensionHostCancellationReasonDto::AuthorityRevoked
    );
}

#[test]
fn outcome_indeterminate_has_a_distinct_protocol_failure() {
    assert_eq!(
        failure_code(super::ExtensionHostFailureKind::OutcomeIndeterminate),
        ExtensionHostFailureCodeDto::OutcomeIndeterminate
    );
}

#[test]
fn output_events_preserve_sequence_fences_and_structured_entry_metadata() {
    let dto = output_event_dto(SequencedExtensionHostOutputEvent {
        sequence: 7,
        event: ExtensionHostOutputEvent {
            context: HostEventContext::new(3, 11),
            operation: HostOutputOperation::Append {
                channel_id: "review".into(),
                text: "ready\n".into(),
                severity: HostOutputSeverity::Warning,
                category: Some("lifecycle".into()),
            },
        },
    });

    assert_eq!(dto.sequence, 7);
    assert_eq!(dto.incarnation, 3);
    assert_eq!(dto.activation_generation, 11);
    assert!(matches!(
        dto.operation,
        ExtensionHostOutputOperationDto::Append {
            channel_id,
            text,
            category: Some(category),
            ..
        } if channel_id == "review" && text == "ready\n" && category == "lifecycle"
    ));
}
