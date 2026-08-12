use crate::ConnectorAccount;
use crate::ConnectorAccountId;
use crate::ConnectorConnectionGeneration;
use crate::ConnectorConnectionState;
use crate::ConnectorConnectionUpdate;
use crate::ConnectorCredentialRef;
use crate::ConnectorDefinition;
use crate::ConnectorErrorKind;
use crate::ConnectorId;
use crate::ConnectorRuntimeBinding;
use crate::ConnectorSnapshot;
use crate::ConnectorSnapshotGeneration;

fn definition(id: &str) -> ConnectorDefinition {
    ConnectorDefinition::new(
        ConnectorId::new(id).unwrap(),
        "GitHub",
        "Connect a GitHub account.",
        ConnectorRuntimeBinding::mcp_server("plugin:acme/github:mcp:github").unwrap(),
    )
    .unwrap()
}

fn connected_account(generation: ConnectorConnectionGeneration) -> ConnectorAccount {
    ConnectorAccount::new(
        ConnectorAccountId::new("octocat").unwrap(),
        "Octocat",
        ConnectorCredentialRef::new("secret:github-octocat").unwrap(),
        generation,
    )
    .unwrap()
}

#[test]
fn snapshot_rejects_duplicate_connector_identity() {
    let error = ConnectorSnapshot::new(
        ConnectorSnapshotGeneration::new(1),
        [definition("acme:github"), definition("acme:github")],
    )
    .unwrap_err();

    assert_eq!(error.kind(), ConnectorErrorKind::DuplicateIdentity);
}

#[test]
fn connection_requires_begin_and_matching_generation() {
    let snapshot = ConnectorSnapshot::new(
        ConnectorSnapshotGeneration::new(1),
        [definition("acme:github")],
    )
    .unwrap();
    let id = ConnectorId::new("acme:github").unwrap();
    let connection_generation = ConnectorConnectionGeneration::new(1);

    let direct_connect_error = snapshot
        .with_connection_update(
            ConnectorSnapshotGeneration::new(2),
            &id,
            ConnectorConnectionUpdate::Connected {
                account: connected_account(connection_generation),
            },
        )
        .unwrap_err();
    assert_eq!(
        direct_connect_error.kind(),
        ConnectorErrorKind::InvalidTransition
    );

    let connecting = snapshot
        .with_connection_update(
            ConnectorSnapshotGeneration::new(2),
            &id,
            ConnectorConnectionUpdate::Begin {
                generation: connection_generation,
            },
        )
        .unwrap();
    let connected = connecting
        .with_connection_update(
            ConnectorSnapshotGeneration::new(3),
            &id,
            ConnectorConnectionUpdate::Connected {
                account: connected_account(connection_generation),
            },
        )
        .unwrap();

    assert!(connected.entry(&id).unwrap().is_ready());
    assert_eq!(connected.ready_entries().count(), 1);
}

#[test]
fn stale_snapshot_or_connection_updates_fail_closed() {
    let id = ConnectorId::new("acme:github").unwrap();
    let snapshot = ConnectorSnapshot::new(
        ConnectorSnapshotGeneration::new(5),
        [definition(id.as_str())],
    )
    .unwrap();

    let stale_snapshot = snapshot
        .with_connection_update(
            ConnectorSnapshotGeneration::new(5),
            &id,
            ConnectorConnectionUpdate::Begin {
                generation: ConnectorConnectionGeneration::new(1),
            },
        )
        .unwrap_err();
    assert_eq!(stale_snapshot.kind(), ConnectorErrorKind::StaleGeneration);

    let connecting = snapshot
        .with_connection_update(
            ConnectorSnapshotGeneration::new(6),
            &id,
            ConnectorConnectionUpdate::Begin {
                generation: ConnectorConnectionGeneration::new(2),
            },
        )
        .unwrap();
    let stale_connection = connecting
        .with_connection_update(
            ConnectorSnapshotGeneration::new(7),
            &id,
            ConnectorConnectionUpdate::Connected {
                account: connected_account(ConnectorConnectionGeneration::new(1)),
            },
        )
        .unwrap_err();
    assert_eq!(stale_connection.kind(), ConnectorErrorKind::StaleGeneration);
}

#[test]
fn disconnect_revokes_runtime_readiness_under_new_generations() {
    let id = ConnectorId::new("acme:github").unwrap();
    let connection_generation = ConnectorConnectionGeneration::new(1);
    let snapshot = ConnectorSnapshot::new(
        ConnectorSnapshotGeneration::new(1),
        [definition(id.as_str())],
    )
    .unwrap()
    .with_connection_update(
        ConnectorSnapshotGeneration::new(2),
        &id,
        ConnectorConnectionUpdate::Begin {
            generation: connection_generation,
        },
    )
    .unwrap()
    .with_connection_update(
        ConnectorSnapshotGeneration::new(3),
        &id,
        ConnectorConnectionUpdate::Connected {
            account: connected_account(connection_generation),
        },
    )
    .unwrap();

    let disconnected = snapshot
        .with_connection_update(
            ConnectorSnapshotGeneration::new(4),
            &id,
            ConnectorConnectionUpdate::Disconnect {
                generation: ConnectorConnectionGeneration::new(2),
            },
        )
        .unwrap();

    assert!(matches!(
        disconnected.entry(&id).unwrap().connection().state(),
        ConnectorConnectionState::Disconnected
    ));
    assert_eq!(disconnected.ready_entries().count(), 0);
}
