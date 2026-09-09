use super::model_choices;
use crate::config::ConfigEditor;
use crate::config::ConfigEditorOutcome;
use crate::config::ConfigSelectionAction;
use crate::config::TerminalSettings;
use crate::config::config_choices;
use crate::nls::Language;
use crate::status::StatusLineSettings;
use crate::test_support::empty_config_snapshot;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use zeta_app_server_protocol::protocol::config::ModelRefDto;
use zeta_app_server_protocol::protocol::config::ProviderConfigDto;
use zeta_app_server_protocol::protocol::model::ModelCatalogEntry;
use zeta_app_server_protocol::protocol::model::ModelListResult;
use zeta_app_server_protocol::protocol::provider::ProviderListResult;
use zeta_protocol::ModelId;
use zeta_protocol::ModelInfo;
use zeta_protocol::ModelOutputTransport;
use zeta_protocol::ModelRef;
use zeta_protocol::ProviderId;

#[test]
fn issue_config_model_picker_uses_configured_catalog_and_returns_to_issues() {
    let mut config = empty_config_snapshot();
    config.revision = 17;
    config.preferred_model = Some(ModelRefDto {
        provider: "openai".into(),
        model: "conversation".into(),
    });
    config.providers.insert(
        "openai".into(),
        ProviderConfigDto {
            provider: "openai".into(),
            custom: None,
            base_url: None,
            max_output_tokens: None,
            model_context: Default::default(),
        },
    );
    let models = ModelListResult {
        models: ["openai", "unconfigured"]
            .into_iter()
            .map(|provider| {
                let id = ModelId::new("small").unwrap();
                let model = ModelRef::new(ProviderId::new(provider).unwrap(), id.clone());
                ModelCatalogEntry::from_info(
                    model,
                    &ModelInfo::new(id, "Small model"),
                    ModelOutputTransport::Unary,
                )
            })
            .collect(),
    };
    let mut editor = ConfigEditor::new(config_choices(
        &config,
        &ProviderListResult { providers: vec![] },
        TerminalSettings::default(),
        StatusLineSettings::default(),
    ));
    let key = |code| KeyEvent::new(code, KeyModifiers::NONE);
    editor.handle_key(key(KeyCode::Up));
    editor.handle_key(key(KeyCode::Up));
    editor.handle_key(key(KeyCode::BackTab));
    editor.handle_key(key(KeyCode::Enter));
    let ConfigEditorOutcome::LoadIssueModels {
        request_id,
        expected_revision,
    } = editor.handle_key(key(KeyCode::Enter))
    else {
        panic!("expected a model catalog request");
    };
    assert_eq!(expected_revision, 17);
    editor.finish_issue_models(
        request_id,
        Ok(model_choices(&config, models, Language::English)),
    );
    assert_eq!(editor.selection().unwrap().title(), "Issue analysis model");
    assert_eq!(editor.selection().unwrap().visible_items().len(), 2);
    assert_eq!(
        editor.selection().unwrap().selected_visible_index(),
        Some(0)
    );
    editor.handle_key(key(KeyCode::Down));
    let ConfigEditorOutcome::Action(ConfigSelectionAction::SetIssues(edit)) =
        editor.handle_key(key(KeyCode::Enter))
    else {
        panic!("expected issue model selection");
    };
    assert!(edit.config.recommend_merge);
    assert_eq!(
        edit.config.analysis_model,
        Some(ModelRefDto {
            provider: "openai".into(),
            model: "small".into()
        })
    );
    assert_eq!(editor.selection().unwrap().title(), "Config");
    assert_eq!(editor.selection().unwrap().active_tab().label(), "Issues");
    assert_eq!(config.preferred_model.unwrap().model, "conversation");
}

#[test]
fn issue_refresh_is_one_horizontal_control_and_preserves_other_settings() {
    let mut config = empty_config_snapshot();
    config.revision = 17;
    config.issues.recommend_merge = false;
    config.issues.analysis_model = Some(ModelRefDto {
        provider: "openai".into(),
        model: "small".into(),
    });
    let choices = || {
        config_choices(
            &config,
            &ProviderListResult { providers: vec![] },
            TerminalSettings::default(),
            StatusLineSettings::default(),
        )
    };
    let focus_refresh = |editor: &mut ConfigEditor| {
        for code in [
            KeyCode::Up,
            KeyCode::Up,
            KeyCode::BackTab,
            KeyCode::Enter,
            KeyCode::Down,
        ] {
            editor.handle_key(KeyEvent::new(code, KeyModifiers::NONE));
        }
    };
    let mut editor = ConfigEditor::new(choices());
    focus_refresh(&mut editor);
    let visible = editor.selection().unwrap().visible_items();
    assert_eq!(visible.len(), 3);
    assert_eq!(visible[1].label(), "Auto refresh");
    assert_eq!(visible[1].description(), Some(" 10m"));

    for (current, key, minutes) in [
        (0, KeyCode::Left, 60),
        (0, KeyCode::Right, 5),
        (5, KeyCode::Right, 10),
        (10, KeyCode::Right, 30),
        (30, KeyCode::Right, 60),
        (60, KeyCode::Right, 0),
        (10, KeyCode::Enter, 30),
        (10, KeyCode::Char(' '), 30),
    ] {
        let mut current_config = config.clone();
        current_config.issues.auto_refresh_minutes = current;
        let mut editor = ConfigEditor::new(config_choices(
            &current_config,
            &ProviderListResult { providers: vec![] },
            TerminalSettings::default(),
            StatusLineSettings::default(),
        ));
        focus_refresh(&mut editor);
        let ConfigEditorOutcome::Action(ConfigSelectionAction::SetIssues(edit)) =
            editor.handle_key(KeyEvent::new(key, KeyModifiers::NONE))
        else {
            panic!("expected issue refresh edit");
        };
        let mut expected = config.issues.clone();
        expected.auto_refresh_minutes = minutes;
        assert_eq!((edit.expected_revision, edit.config), (17, expected));
    }
}
