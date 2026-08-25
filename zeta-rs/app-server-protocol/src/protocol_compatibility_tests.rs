use crate::protocol::common::{SchemaHash, ServerInfo};
use crate::protocol::initialize::{
    APP_SERVER_PROTOCOL_MAJOR, CapabilityContract, InitializeResult, ProtocolCompatibilityError,
    ProtocolVersion, REQUIRED_SESSION_CAPABILITIES, ServerCapabilities, ensure_protocol_compatible,
};
use std::collections::BTreeMap;

fn initialization() -> InitializeResult {
    InitializeResult {
        server_info: ServerInfo {
            name: "zeta-app-server".into(),
            version: "test".into(),
        },
        protocol_version: ProtocolVersion::current(),
        schema_hash: SchemaHash("different-schema-is-diagnostic-only".into()),
        capabilities: ServerCapabilities {
            sessions: true,
            threads: true,
            turns: true,
            contracts: BTreeMap::from([
                ("sessions".into(), CapabilityContract::current()),
                ("threads".into(), CapabilityContract::current()),
                ("turns".into(), CapabilityContract::current()),
            ]),
            ..ServerCapabilities::default()
        },
        slash_commands: Vec::new(),
    }
}

#[test]
fn schema_drift_does_not_make_a_compatible_protocol_fatal() {
    assert_eq!(
        ensure_protocol_compatible(&initialization(), REQUIRED_SESSION_CAPABILITIES),
        Ok(())
    );
}

#[test]
fn protocol_major_mismatch_is_fatal() {
    let mut initialized = initialization();
    initialized.protocol_version.major = APP_SERVER_PROTOCOL_MAJOR + 1;

    assert!(matches!(
        ensure_protocol_compatible(&initialized, REQUIRED_SESSION_CAPABILITIES),
        Err(ProtocolCompatibilityError::MajorVersion { .. })
    ));
}

#[test]
fn missing_or_disabled_required_capability_is_fatal() {
    let mut missing = initialization();
    missing.capabilities.contracts.remove("turns");
    assert!(matches!(
        ensure_protocol_compatible(&missing, REQUIRED_SESSION_CAPABILITIES),
        Err(ProtocolCompatibilityError::MissingCapability { name: "turns", .. })
    ));

    let mut disabled = initialization();
    disabled.capabilities.turns = false;
    assert!(matches!(
        ensure_protocol_compatible(&disabled, REQUIRED_SESSION_CAPABILITIES),
        Err(ProtocolCompatibilityError::MissingCapability { name: "turns", .. })
    ));
}

#[test]
fn unsupported_required_capability_version_is_fatal() {
    let mut initialized = initialization();
    initialized
        .capabilities
        .contracts
        .insert("turns".into(), CapabilityContract { version: 2 });

    assert!(matches!(
        ensure_protocol_compatible(&initialized, REQUIRED_SESSION_CAPABILITIES),
        Err(ProtocolCompatibilityError::CapabilityVersion {
            name: "turns",
            received: 2,
            ..
        })
    ));
}
