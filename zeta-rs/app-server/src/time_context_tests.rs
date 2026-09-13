use super::*;
use zeta_config::ConfigCommandRequest;
use zeta_config::ConfigRevision;
use zeta_config::PreferencesUpdate;
use zeta_config::TimeContextConfig;
use zeta_config::UserConfigCommand;
use zeta_protocol::CommandId;
use zeta_protocol::Patch;

#[test]
fn time_context_profile_policy_changes_are_seen_without_replacing_the_provider() {
    let dir = tempfile::tempdir().unwrap();
    let config = Arc::new(ConfigStore::open(dir.path().join("state.sqlite3")).unwrap());
    let provider = ConfigTimeContext::new(config.clone());
    for (index, mode) in [
        TimeContextMode::Date,
        TimeContextMode::Time,
        TimeContextMode::Off,
    ]
    .into_iter()
    .enumerate()
    {
        config
            .apply(ConfigCommandRequest {
                command_id: CommandId::new(format!("time-{index}")).unwrap(),
                expected_revision: ConfigRevision::new(index as u64),
                command: UserConfigCommand::UpdatePreferences(PreferencesUpdate {
                    time_context: Patch::Value(TimeContextConfig {
                        mode,
                        time_zone: Some("Asia/Tokyo".into()),
                    }),
                    ..PreferencesUpdate::default()
                }),
            })
            .unwrap();
        let snapshot = provider.snapshot().unwrap();
        if mode == TimeContextMode::Off {
            assert!(snapshot.is_none());
        } else {
            let snapshot = snapshot.unwrap();
            assert_eq!(snapshot.mode, mode);
            assert_eq!(snapshot.time_zone, "Asia/Tokyo");
            assert_eq!(snapshot.origin, TimeZoneOrigin::Configured);
            let rendered = zeta_agent_environment::TimeSnapshot::new(snapshot)
                .unwrap()
                .render();
            assert!(rendered.contains(if mode == TimeContextMode::Date {
                "date:"
            } else {
                "time:"
            }));
        }
    }
}
