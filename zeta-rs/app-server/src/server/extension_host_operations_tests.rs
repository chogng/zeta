use super::cancellation_reason;
use super::failure_code;
use zeta_app_server_protocol::protocol::extension_host::ExtensionHostCancellationReasonDto;
use zeta_app_server_protocol::protocol::extension_host::ExtensionHostFailureCodeDto;
use zeta_editor_extension_host::CancelReason;

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
