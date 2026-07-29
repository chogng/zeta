use super::*;

#[test]
fn terminal_environment_excludes_unrelated_process_secrets() {
    assert!(allowed_environment_key("PATH"));
    assert!(allowed_environment_key("SystemRoot"));
    assert!(allowed_environment_key("LC_ALL"));
    assert!(!allowed_environment_key("OPENAI_API_KEY"));
    assert!(!allowed_environment_key("AWS_SECRET_ACCESS_KEY"));
}

#[test]
fn discovered_profiles_have_one_default_and_unique_programs() {
    let catalog = TerminalProfileCatalog::discover();
    let profiles = catalog.list();
    assert_eq!(
        profiles.iter().filter(|profile| profile.is_default).count(),
        1
    );
    let programs = catalog
        .profiles
        .iter()
        .map(|profile| normalized_program(&profile.program))
        .collect::<HashSet<_>>();
    assert_eq!(programs.len(), profiles.len());
}
