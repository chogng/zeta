use super::*;

#[test]
fn updates_survive_reopen() {
    let path = std::env::temp_dir().join(format!("zeta-config-{}.json", std::process::id()));
    let store = ConfigStore::open(&path).unwrap();
    store
        .update(ConfigUpdate {
            preferred_model: Some(Some("model".into())),
            theme: Some(Some(Theme::Dark)),
        })
        .unwrap();
    assert_eq!(
        ConfigStore::open(&path)
            .unwrap()
            .read()
            .unwrap()
            .preferred_model,
        Some("model".into())
    );
    let _ = std::fs::remove_file(path);
}
