use super::*;

#[test]
fn provisioning_frame_round_trips() {
    let frame = WindowsSandboxProvisioningFrame {
        version: SANDBOX_SERVICE_PROTOCOL_VERSION,
        message: WindowsSandboxProvisioningMessage::Provision(WindowsSandboxProvisioningRequest {
            dir: PathBuf::from(r"C:\work"),
            program: PathBuf::from(r"C:\tools\rg.exe"),
            access: WindowsSandboxProvisioningAccess::DirectoryWrite,
        }),
    };
    let mut encoded = Vec::new();
    write_provisioning_frame(&mut encoded, &frame).unwrap();
    assert_eq!(
        read_provisioning_frame(encoded.as_slice()).unwrap(),
        Some(frame)
    );
}

#[test]
fn provisioning_frame_rejects_oversized_payload() {
    let mut encoded = Vec::from(((4096_u32) + 1).to_le_bytes());
    encoded.resize(encoded.len() + 4097, 0);
    assert!(read_provisioning_frame(encoded.as_slice()).is_err());
}
