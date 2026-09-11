use super::*;
use crate::policy::FilesystemSection;
use crate::policy::SandboxPolicy;

#[test]
fn host_read_ceiling_does_not_extend_acl_mutation_authority() {
    let temp = tempfile::tempdir().unwrap();
    let work = std::fs::canonicalize(temp.path()).unwrap();
    let policy = SandboxPolicy {
        version: "0.8.0-alpha".into(),
        filesystem: Some(FilesystemSection {
            readwrite_paths: vec![work.to_str().unwrap().into()],
            ..Default::default()
        }),
        network: None,
        ui: None,
        timeout_ms: None,
    };
    let mut request = crate::build_request(&policy, None).unwrap();
    request
        .set_host_filesystem(HostFilesystemAccess::ReadOnly)
        .unwrap();
    request
        .permit_host_acl_changes(std::slice::from_ref(&work))
        .unwrap();
    let authority = request.inner.host_acl_scope.as_ref().unwrap();
    authority.check(&work).unwrap();
    assert!(authority.check(work.parent().unwrap()).is_err());
    #[cfg(windows)]
    {
        assert!(request.inner.policy.readonly_paths.is_empty());
        assert!(!request.inner.host_filesystem_roots.is_empty());
    }
}

#[test]
fn managed_proxy_never_opens_general_ingress() {
    use crate::policy::NetworkSection;
    let network = NetworkSection::managed_proxy(3128.try_into().unwrap());
    assert_eq!(
        network.egress.as_ref().unwrap().default,
        Some(crate::NetworkAction::Deny)
    );
    let ingress = network.ingress.as_ref().unwrap();
    assert_eq!(ingress.default, Some(crate::NetworkAction::Deny));
    assert_eq!(ingress.host_loopback, Some(crate::NetworkAction::Deny));
    #[cfg(windows)]
    {
        let rules = network.egress.as_ref().unwrap().allow.as_ref().unwrap();
        assert_eq!(rules.len(), 1);
        let rule = &rules[0];
        assert_eq!(rule.to.as_ref().unwrap()[0].cidr, "127.0.0.1/32");
        let port = &rule.ports.as_ref().unwrap()[0];
        assert_eq!(port.protocol, Some(crate::NetworkProtocol::Tcp));
        assert_eq!(port.port, Some(3128));
        assert_eq!(port.end_port, None);
        assert!(network.runtime_config.is_none());
    }
    #[cfg(not(windows))]
    assert_eq!(
        network.runtime_config.unwrap().network_proxy.as_deref(),
        Some("http://127.0.0.1:3128")
    );
}
