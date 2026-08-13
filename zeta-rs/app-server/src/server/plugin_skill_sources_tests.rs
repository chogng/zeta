use std::fs;
use tempfile::tempdir;
use zeta_plugins::LocalPluginPackage;
use zeta_plugins::PluginActivationAuthority;
use zeta_plugins::PluginAuthorityCommand;
use zeta_plugins::PluginAuthorityCommandId;
use zeta_plugins::PluginAuthorityCommandRequest;
use zeta_skills_extension::DynamicSkillSourceProvider;

use super::PluginSkillSourceProvider;

#[test]
fn effective_plugin_contributions_project_only_declared_exact_skill_directories() {
    let root = tempdir().unwrap();
    let source = root.path().join("source");
    fs::create_dir_all(source.join(".zeta-plugin")).unwrap();
    fs::create_dir_all(source.join("skills/review")).unwrap();
    fs::create_dir_all(source.join("skills/undeclared")).unwrap();
    fs::write(
        source.join("skills/review/SKILL.md"),
        "---\nname: review\ndescription: Review code.\n---\n# Review",
    )
    .unwrap();
    fs::write(
        source.join("skills/undeclared/SKILL.md"),
        "---\nname: undeclared\ndescription: Hidden.\n---\n# Hidden",
    )
    .unwrap();
    fs::write(
        source.join(".zeta-plugin/plugin.json"),
        r#"{
            "schemaVersion": 1,
            "id": "acme/review",
            "version": "1.0.0",
            "displayName": "Review",
            "compatibility": {"zeta": ">=0.1.0"},
            "contributions": {"skills": [{"id": "review", "path": "skills/review"}]}
        }"#,
    )
    .unwrap();
    let local = LocalPluginPackage::load(&source).unwrap();
    let authority = PluginActivationAuthority::open(root.path().join("profile")).unwrap();
    let installed = authority
        .install_local(PluginAuthorityCommandId::new("install").unwrap(), 0, &local)
        .unwrap()
        .package;
    for (id, command) in [
        (
            "grant",
            PluginAuthorityCommand::Grant {
                package: installed.clone(),
            },
        ),
        (
            "enable",
            PluginAuthorityCommand::Enable {
                package: installed.clone(),
            },
        ),
    ] {
        authority
            .apply(PluginAuthorityCommandRequest {
                command_id: PluginAuthorityCommandId::new(id).unwrap(),
                expected_revision: authority.snapshot().revision(),
                command,
            })
            .unwrap();
    }

    let snapshot = PluginSkillSourceProvider::new(authority)
        .snapshot()
        .unwrap();
    let catalog = zeta_skills::SkillCatalog::discover(snapshot.roots).unwrap();

    assert_eq!(catalog.snapshot().list().len(), 1);
    assert_eq!(catalog.snapshot().list()[0].id().name.as_str(), "review");
    assert_eq!(
        catalog.snapshot().list()[0].id().source.as_str(),
        "plugin-acme/review:skill-source:review"
    );
}
