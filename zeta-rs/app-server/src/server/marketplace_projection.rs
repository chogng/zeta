use zeta_app_server_protocol::protocol::marketplace::MarketplaceAcquiredCapabilityDto;
use zeta_app_server_protocol::protocol::marketplace::MarketplaceActivationSpecDto;
use zeta_app_server_protocol::protocol::marketplace::MarketplaceArtifactHandleDto;
use zeta_app_server_protocol::protocol::marketplace::MarketplaceAvailableCapabilityDto;
use zeta_app_server_protocol::protocol::marketplace::MarketplaceCapabilityDescriptorDto;
use zeta_app_server_protocol::protocol::marketplace::MarketplaceCapabilityKindDto;
use zeta_app_server_protocol::protocol::marketplace::MarketplaceCapabilityLeaseDto;
use zeta_app_server_protocol::protocol::marketplace::MarketplaceCapabilityRefDto;
use zeta_app_server_protocol::protocol::marketplace::MarketplaceConnectorActivationSpecDto;
use zeta_app_server_protocol::protocol::marketplace::MarketplaceExecutableActivationSpecDto;
use zeta_app_server_protocol::protocol::marketplace::MarketplaceExecutableRuntimeDto;
use zeta_app_server_protocol::protocol::marketplace::MarketplaceInstallationStateDto;
use zeta_app_server_protocol::protocol::marketplace::MarketplaceInstalledPackageDto;
use zeta_app_server_protocol::protocol::marketplace::MarketplaceLanguageActivationSpecDto;
use zeta_app_server_protocol::protocol::marketplace::MarketplaceMcpActivationSpecDto;
use zeta_app_server_protocol::protocol::marketplace::MarketplaceMcpTransportDto;
use zeta_app_server_protocol::protocol::marketplace::MarketplacePackageDetailsDto;
use zeta_app_server_protocol::protocol::marketplace::MarketplacePackageRefDto;
use zeta_app_server_protocol::protocol::marketplace::MarketplacePackageSourceDto;
use zeta_app_server_protocol::protocol::marketplace::MarketplacePackageSummaryDto;
use zeta_app_server_protocol::protocol::marketplace::MarketplaceResourceContentDto;
use zeta_app_server_protocol::protocol::marketplace::MarketplaceResourceRefDto;
use zeta_app_server_protocol::protocol::marketplace::MarketplaceSearchResult;
use zeta_app_server_protocol::protocol::marketplace::MarketplaceSkillActivationSpecDto;
use zeta_app_server_protocol::protocol::marketplace::MarketplaceThemeActivationSpecDto;
use zeta_app_server_protocol::protocol::marketplace::MarketplaceUpstreamReferenceDto;
use zeta_app_server_protocol::protocol::marketplace::MarketplaceUpstreamRegistryDto;
use zeta_marketplace_client::AcquiredCapability;
use zeta_marketplace_client::ActivationSpec;
use zeta_marketplace_client::ArtifactHandle;
use zeta_marketplace_client::AvailableCapability;
use zeta_marketplace_client::CapabilityDescriptor;
use zeta_marketplace_client::CapabilityKind;
use zeta_marketplace_client::CapabilityLease;
use zeta_marketplace_client::CapabilityRef;
use zeta_marketplace_client::ConnectorActivationSpec;
use zeta_marketplace_client::ExecutableActivationSpec;
use zeta_marketplace_client::ExecutableRuntime;
use zeta_marketplace_client::InstallationState;
use zeta_marketplace_client::InstalledPackage;
use zeta_marketplace_client::LanguageActivationSpec;
use zeta_marketplace_client::McpActivationSpec;
use zeta_marketplace_client::McpTransportSpec;
use zeta_marketplace_client::PackageDetails;
use zeta_marketplace_client::PackageRef;
use zeta_marketplace_client::PackageSource;
use zeta_marketplace_client::PackageSummary;
use zeta_marketplace_client::ResourceContent;
use zeta_marketplace_client::ResourceRef;
use zeta_marketplace_client::SearchPackagesResult;
use zeta_marketplace_client::SkillActivationSpec;
use zeta_marketplace_client::ThemeActivationSpec;
use zeta_marketplace_client::UpstreamReference;
use zeta_marketplace_client::UpstreamRegistry;

pub(super) fn search_result(value: SearchPackagesResult) -> MarketplaceSearchResult {
    MarketplaceSearchResult {
        packages: value.packages.into_iter().map(package_summary).collect(),
    }
}

pub(super) fn package_details(value: PackageDetails) -> MarketplacePackageDetailsDto {
    MarketplacePackageDetailsDto {
        package: package_ref(value.package),
        package_type: value.package_type,
        display_name: value.display_name,
        description: value.description,
        license: value.license,
        source: package_source(value.source),
        upstream: value.upstream.map(upstream_reference),
        capabilities: value
            .capabilities
            .into_iter()
            .map(available_capability)
            .collect(),
    }
}

fn package_source(value: PackageSource) -> MarketplacePackageSourceDto {
    match value {
        PackageSource::Official => MarketplacePackageSourceDto::Official,
        PackageSource::ThirdParty => MarketplacePackageSourceDto::ThirdParty,
    }
}

fn upstream_reference(value: UpstreamReference) -> MarketplaceUpstreamReferenceDto {
    MarketplaceUpstreamReferenceDto {
        registry: match value.registry {
            UpstreamRegistry::OfficialMcp => MarketplaceUpstreamRegistryDto::OfficialMcp,
        },
        name: value.name,
        version: value.version,
        record_url: value.record_url,
        repository_url: value.repository_url,
    }
}

pub(super) fn artifact_handle(value: ArtifactHandle) -> MarketplaceArtifactHandleDto {
    MarketplaceArtifactHandleDto {
        id: value.id,
        package: package_ref(value.package),
    }
}

pub(super) fn installed_package(value: InstalledPackage) -> MarketplaceInstalledPackageDto {
    MarketplaceInstalledPackageDto {
        installation_id: value.installation_id,
        package: package_ref(value.package),
        state: match value.state {
            InstallationState::Installed => MarketplaceInstallationStateDto::Installed,
            InstallationState::PendingRemoval => MarketplaceInstallationStateDto::PendingRemoval,
        },
        capabilities: value
            .capabilities
            .into_iter()
            .map(capability_descriptor)
            .collect(),
    }
}

pub(super) fn acquired_capability(value: AcquiredCapability) -> MarketplaceAcquiredCapabilityDto {
    MarketplaceAcquiredCapabilityDto {
        lease: capability_lease(value.lease),
        spec: activation_spec(value.spec),
    }
}

pub(super) fn resource_content(value: ResourceContent) -> MarketplaceResourceContentDto {
    MarketplaceResourceContentDto {
        media_type: value.media_type,
        data_base64: value.data_base64,
    }
}

fn package_summary(value: PackageSummary) -> MarketplacePackageSummaryDto {
    MarketplacePackageSummaryDto {
        id: value.id,
        version: value.version,
        package_type: value.package_type,
        display_name: value.display_name,
        description: value.description,
    }
}

fn package_ref(value: PackageRef) -> MarketplacePackageRefDto {
    MarketplacePackageRefDto {
        id: value.id,
        version: value.version,
        digest: value.digest,
    }
}

fn capability_ref(value: CapabilityRef) -> MarketplaceCapabilityRefDto {
    MarketplaceCapabilityRefDto { id: value.id }
}

fn resource_ref(value: ResourceRef) -> MarketplaceResourceRefDto {
    MarketplaceResourceRefDto { id: value.id }
}

fn capability_kind(value: CapabilityKind) -> MarketplaceCapabilityKindDto {
    match value {
        CapabilityKind::Skill => MarketplaceCapabilityKindDto::Skill,
        CapabilityKind::Mcp => MarketplaceCapabilityKindDto::Mcp,
        CapabilityKind::Connector => MarketplaceCapabilityKindDto::Connector,
        CapabilityKind::Theme => MarketplaceCapabilityKindDto::Theme,
        CapabilityKind::Language => MarketplaceCapabilityKindDto::Language,
        CapabilityKind::Executable => MarketplaceCapabilityKindDto::Executable,
        CapabilityKind::Asset => MarketplaceCapabilityKindDto::Asset,
    }
}

fn capability_descriptor(value: CapabilityDescriptor) -> MarketplaceCapabilityDescriptorDto {
    MarketplaceCapabilityDescriptorDto {
        reference: capability_ref(value.reference),
        kind: capability_kind(value.kind),
        id: value.id,
        contract_version: value.contract_version,
        permissions: value.permissions,
        authentication_provider: value.authentication_provider,
    }
}

fn available_capability(value: AvailableCapability) -> MarketplaceAvailableCapabilityDto {
    MarketplaceAvailableCapabilityDto {
        kind: capability_kind(value.kind),
        id: value.id,
        contract_version: value.contract_version,
        permissions: value.permissions,
        authentication_provider: value.authentication_provider,
    }
}

fn capability_lease(value: CapabilityLease) -> MarketplaceCapabilityLeaseDto {
    MarketplaceCapabilityLeaseDto {
        id: value.id,
        capability: capability_ref(value.capability),
        installation_id: value.installation_id,
    }
}

fn activation_spec(value: ActivationSpec) -> MarketplaceActivationSpecDto {
    match value {
        ActivationSpec::Skill(value) => {
            MarketplaceActivationSpecDto::Skill(skill_activation(value))
        }
        ActivationSpec::Mcp(value) => MarketplaceActivationSpecDto::Mcp(mcp_activation(value)),
        ActivationSpec::Connector(value) => {
            MarketplaceActivationSpecDto::Connector(connector_activation(value))
        }
        ActivationSpec::Theme(value) => {
            MarketplaceActivationSpecDto::Theme(theme_activation(value))
        }
        ActivationSpec::Language(value) => {
            MarketplaceActivationSpecDto::Language(language_activation(value))
        }
        ActivationSpec::Executable(value) => {
            MarketplaceActivationSpecDto::Executable(executable_activation(value))
        }
    }
}

fn skill_activation(value: SkillActivationSpec) -> MarketplaceSkillActivationSpecDto {
    MarketplaceSkillActivationSpecDto {
        contract_version: value.contract_version,
        resource: resource_ref(value.resource),
    }
}

fn mcp_activation(value: McpActivationSpec) -> MarketplaceMcpActivationSpecDto {
    MarketplaceMcpActivationSpecDto {
        contract_version: value.contract_version,
        transport: match value.transport {
            McpTransportSpec::Stdio { executable, args } => MarketplaceMcpTransportDto::Stdio {
                executable: resource_ref(executable),
                args,
            },
            McpTransportSpec::StreamableHttp { url } => {
                MarketplaceMcpTransportDto::StreamableHttp { url }
            }
        },
        network_hosts: value.network_hosts,
    }
}

fn connector_activation(value: ConnectorActivationSpec) -> MarketplaceConnectorActivationSpecDto {
    MarketplaceConnectorActivationSpecDto {
        contract_version: value.contract_version,
        authentication_provider: value.authentication_provider,
        mcp: value.mcp.map(capability_ref),
    }
}

fn theme_activation(value: ThemeActivationSpec) -> MarketplaceThemeActivationSpecDto {
    MarketplaceThemeActivationSpecDto {
        contract_version: value.contract_version,
        manifest: resource_ref(value.manifest),
    }
}

fn language_activation(value: LanguageActivationSpec) -> MarketplaceLanguageActivationSpecDto {
    MarketplaceLanguageActivationSpecDto {
        contract_version: value.contract_version,
        manifest: resource_ref(value.manifest),
    }
}

fn executable_activation(
    value: ExecutableActivationSpec,
) -> MarketplaceExecutableActivationSpecDto {
    MarketplaceExecutableActivationSpecDto {
        contract_version: value.contract_version,
        runtime: match value.runtime {
            ExecutableRuntime::Direct => MarketplaceExecutableRuntimeDto::Direct,
            ExecutableRuntime::Node => MarketplaceExecutableRuntimeDto::Node,
        },
        entrypoint: resource_ref(value.entrypoint),
    }
}
