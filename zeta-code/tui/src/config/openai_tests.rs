use super::*;
use crate::config::TerminalSettings;
use crate::status::StatusLineSettings;

fn panel() -> Panel {
    Panel::new(Settings::new(
        &crate::test_support::empty_config_snapshot(),
        &ProviderListResult {
            providers: Vec::new(),
        },
    ))
}
fn key(panel: &mut Panel, code: KeyCode) -> ConfigEditorOutcome {
    panel.handle_key(KeyEvent::new(code, KeyModifiers::NONE))
}
fn request(outcome: ConfigEditorOutcome) -> Request {
    let ConfigEditorOutcome::Action(ConfigSelectionAction::Connection(request)) = outcome else {
        panic!("expected connection request")
    };
    request
}
fn success(panel: &mut Panel, request: &Request) {
    let mut config = crate::test_support::empty_config_snapshot();
    config.revision = request.revision + 1;
    config.providers = panel.settings.configs.clone();
    config
        .providers
        .insert(request.config.provider.clone(), request.config.clone());
    let choices = super::super::config_choices(
        &config,
        &ProviderListResult {
            providers: Vec::new(),
        },
        TerminalSettings::default(),
        StatusLineSettings::default(),
    );
    panel.complete(Reply {
        id: request.id.clone(),
        result: Ok((choices, None)),
    });
}
fn draft(panel: &mut Panel) {
    panel.select_tab(panel.tabs.tabs().len() - 1);
    for value in ["My service", "https://example.test/v1", "test-key"] {
        key(panel, KeyCode::Enter);
        panel.handle_paste(value.into());
        assert!(matches!(
            key(panel, KeyCode::Enter),
            ConfigEditorOutcome::Consumed
        ));
        assert!(!panel.form().unwrap().editing());
        key(panel, KeyCode::Down);
    }
    key(panel, KeyCode::Enter);
}

fn create(panel: &mut Panel) -> Request {
    draft(panel);
    key(panel, KeyCode::Enter);
    key(panel, KeyCode::Down);
    request(key(panel, KeyCode::Enter))
}

#[test]
fn form_and_nested_list_return_up_to_the_shared_tab_list() {
    let mut panel = panel();

    assert!(!panel.form().unwrap().editing());
    key(&mut panel, KeyCode::Up);
    assert_eq!(panel.focus, PanelFocus::Tabs);
    assert!(panel.key_hints().starts_with("Tab/Shift+Tab to switch"));

    assert!(matches!(
        key(&mut panel, KeyCode::Tab),
        ConfigEditorOutcome::Action(ConfigSelectionAction::OpenSubscription)
    ));
    assert_eq!(panel.tabs.active_index(), Some(1));
    assert_eq!(panel.focus, PanelFocus::Tabs);

    key(&mut panel, KeyCode::Down);
    assert_eq!(panel.focus, PanelFocus::Content);
    key(&mut panel, KeyCode::Up);
    assert_eq!(panel.focus, PanelFocus::Tabs);
}

#[test]
fn down_stops_at_the_last_form_action() {
    let mut panel = panel();
    let form = panel.form_mut().unwrap();
    form.focus(4);

    key(&mut panel, KeyCode::Down);

    assert_eq!(panel.focus, PanelFocus::Content);
    assert_eq!(panel.form().unwrap().focus, 4);
}

#[test]
fn enter_confirms_each_draft_field_and_creation_preserves_new_tab() {
    let mut panel = panel();
    draft(&mut panel);
    assert_eq!(panel.form().unwrap().focus, 3);
    assert!(panel.form().unwrap().editing());
    assert!(panel.pending.is_none());
    assert_eq!(
        panel.form().unwrap().protocol,
        CustomProviderProtocolDto::Responses
    );
    key(&mut panel, KeyCode::Right);
    key(&mut panel, KeyCode::Enter);
    assert_eq!(panel.form().unwrap().focus, 3);
    key(&mut panel, KeyCode::Down);
    assert_eq!(panel.form().unwrap().focus, 4);
    assert!(!panel.form().unwrap().editing());
    let request = request(key(&mut panel, KeyCode::Enter));
    assert_eq!(
        request.config.custom.as_ref().unwrap().protocol,
        CustomProviderProtocolDto::ChatCompletions
    );
    assert_eq!(
        request.key.clone().unwrap().into_parts(),
        (request.config.provider.clone(), "test-key".into())
    );
    success(&mut panel, &request);
    assert_eq!(
        panel
            .tabs
            .tabs()
            .iter()
            .map(|tab| tab.label.as_str())
            .collect::<Vec<_>>(),
        vec![
            "Official API key",
            "ChatGPT subscription",
            "My service",
            "New custom provider"
        ]
    );
    assert_eq!(panel.tabs.active_tab().unwrap().id, request.config.provider);
    assert!(!panel.form().unwrap().draft);
    assert!(panel.form().unwrap().key.query().is_empty());
    panel.select_tab(3);
    assert!(panel.form().unwrap().name.query().is_empty());
}

#[test]
fn saved_field_exits_editing_only_after_success_and_failure_keeps_input() {
    let mut panel = panel();
    key(&mut panel, KeyCode::Enter);
    panel.handle_paste("test-key".into());
    let first = request(key(&mut panel, KeyCode::Enter));
    assert_eq!(panel.form().unwrap().focus, 2);
    panel.complete(Reply {
        id: first.id,
        result: Err("Secret store unavailable".into()),
    });
    assert!(panel.form().unwrap().editing());
    assert_eq!(panel.form().unwrap().key.query(), "test-key");
    let retry = request(key(&mut panel, KeyCode::Enter));
    success(&mut panel, &retry);
    assert_eq!(panel.form().unwrap().focus, 2);
    assert!(!panel.form().unwrap().editing());
    assert!(panel.pending.is_none());
}

#[test]
fn tab_switches_from_editing_without_saving_and_preserves_drafts() {
    let mut panel = panel();
    key(&mut panel, KeyCode::Enter);
    panel.handle_paste("unconfirmed-key".into());
    assert!(panel.key_hints().contains("Tab/Shift+Tab to switch"));
    assert!(matches!(
        key(&mut panel, KeyCode::Tab),
        ConfigEditorOutcome::Action(ConfigSelectionAction::OpenSubscription)
    ));
    assert_eq!(panel.tabs.active_index(), Some(1));
    assert_eq!(panel.focus, PanelFocus::Content);
    key(&mut panel, KeyCode::BackTab);
    assert_eq!(panel.form().unwrap().key.query(), "unconfirmed-key");
    assert!(panel.form().unwrap().editing());
    assert!(panel.pending.is_none());
    key(&mut panel, KeyCode::Esc);
    assert!(panel.form().unwrap().key.query().is_empty());
    draft(&mut panel);
    let id = panel.tabs.active_tab().unwrap().id.clone();
    key(&mut panel, KeyCode::Tab);
    assert_eq!(panel.tabs.active_index(), Some(0));
    key(&mut panel, KeyCode::BackTab);
    assert_eq!(panel.tabs.active_tab().unwrap().id, id);
    assert_eq!(panel.form().unwrap().name.query(), "My service");
    assert_eq!(panel.form().unwrap().key.query(), "test-key");
    assert!(panel.form().unwrap().editing());
    assert!(panel.pending.is_none());
}

#[test]
fn invalid_url_stays_in_field_and_escape_restores_confirmed_value() {
    let mut panel = panel();
    panel.select_tab(2);
    key(&mut panel, KeyCode::Enter);
    panel.handle_paste("Example".into());
    key(&mut panel, KeyCode::Enter);
    key(&mut panel, KeyCode::Down);
    key(&mut panel, KeyCode::Enter);
    panel.handle_paste("not-a-url".into());
    key(&mut panel, KeyCode::Enter);
    assert_eq!(panel.form().unwrap().focus, 1);
    assert!(panel.form().unwrap().editing());
    assert!(panel.pending.is_none());
    key(&mut panel, KeyCode::Esc);
    assert!(panel.form().unwrap().url.query().is_empty());
    assert!(!panel.form().unwrap().editing());
}

#[test]
fn late_creation_reply_does_not_switch_back_to_its_tab() {
    let mut panel = panel();
    let request = create(&mut panel);
    key(&mut panel, KeyCode::Tab);
    assert_eq!(panel.tabs.active_index(), Some(0));
    success(&mut panel, &request);
    assert_eq!(panel.tabs.active_index(), Some(0));
    assert_eq!(panel.tabs.tabs().len(), 4);
}

#[test]
fn unchanged_existing_field_exits_editing_without_request() {
    let mut panel = panel();
    key(&mut panel, KeyCode::Enter);
    // Existing empty API key means leave the saved secret unchanged.
    assert!(matches!(
        key(&mut panel, KeyCode::Enter),
        ConfigEditorOutcome::Consumed
    ));
    assert_eq!(panel.form().unwrap().focus, 2);
    assert!(!panel.form().unwrap().editing());
    assert!(panel.pending.is_none());
}

#[test]
fn existing_name_confirmation_saves_and_keeps_the_current_field_selected() {
    let mut panel = panel();
    let created = create(&mut panel);
    success(&mut panel, &created);
    for _ in 0..panel.form().unwrap().focus {
        key(&mut panel, KeyCode::Up);
    }
    key(&mut panel, KeyCode::Enter);
    panel.handle_paste(" renamed".into());
    let renamed = request(key(&mut panel, KeyCode::Enter));
    assert_eq!(panel.form().unwrap().focus, 0);
    success(&mut panel, &renamed);
    assert_eq!(panel.form().unwrap().focus, 0);
    assert!(!panel.form().unwrap().editing());
    key(&mut panel, KeyCode::Down);
    assert!(!panel.form().unwrap().editing());
    key(&mut panel, KeyCode::Enter);
    panel.handle_paste("/gateway".into());
    assert!(panel.form().unwrap().url.query().ends_with("/gateway"));
    assert_eq!(
        panel.form().unwrap().saved.provider,
        created.config.provider
    );
}

#[test]
fn openai_form_character_output_masks_key_and_keeps_focused_field_visible() {
    let mut panel = panel();
    draft(&mut panel);
    for (width, height) in [(80, 24), (36, 14)] {
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(width, height)).unwrap();
        let app = crate::app::App::new();
        terminal
            .draw(|frame| {
                let tabs = panel.tab_rows(width);
                panel.draw_tabs(
                    frame,
                    Rect::new(0, 0, width, tabs),
                    None,
                    None,
                    app.render_context(),
                );
                panel.draw_body(
                    frame,
                    Rect::new(0, tabs, width, height - tabs),
                    app.render_context(),
                );
            })
            .unwrap();
        let buffer = terminal.backend().buffer();
        let output = (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(output.contains("API protocol"));
        assert!(!output.contains("test-key"));
        assert!(output.contains("Responses"));
        insta::assert_snapshot!(format!("openai_form_{width}x{height}"), output);
    }
}

#[test]
fn entering_and_moving_between_fields_does_not_accept_text_until_enter() {
    let mut panel = panel();
    assert!(!panel.form().unwrap().editing());
    panel.handle_paste("ignored".into());
    key(&mut panel, KeyCode::Char('x'));
    assert!(panel.form().unwrap().key.query().is_empty());
    key(&mut panel, KeyCode::Down);
    assert_eq!(panel.form().unwrap().focus, 4);
    key(&mut panel, KeyCode::Up);
    assert_eq!(panel.form().unwrap().focus, 2);
    assert!(!panel.form().unwrap().editing());
    key(&mut panel, KeyCode::Enter);
    panel.handle_paste("draft-key".into());
    key(&mut panel, KeyCode::Esc);
    assert!(panel.form().unwrap().key.query().is_empty());
    assert!(!panel.form().unwrap().editing());
    assert_eq!(panel.focus, PanelFocus::Content);
    assert!(panel.pending.is_none());
}

#[test]
fn fetching_clears_old_models_and_failure_can_be_retried_without_saving() {
    let mut panel = panel();
    panel.form_mut().unwrap().models = vec!["old-model".into()];
    key(&mut panel, KeyCode::Down);
    let first = request(key(&mut panel, KeyCode::Enter));
    assert_eq!(first.operation, Operation::FetchModels);
    assert!(first.key.is_none());
    assert!(panel.form().unwrap().models.is_empty());
    assert_eq!(panel.form().unwrap().message, "Fetching models…");
    let choices = || {
        super::super::config_choices(
            &crate::test_support::empty_config_snapshot(),
            &ProviderListResult {
                providers: Vec::new(),
            },
            TerminalSettings::default(),
            StatusLineSettings::default(),
        )
    };
    panel.complete(Reply {
        id: first.id.clone(),
        result: Ok((
            choices(),
            Some(Err("Failed to fetch models · Permission denied".into())),
        )),
    });
    assert!(panel.form().unwrap().message.contains("Permission denied"));
    assert!(panel.form().unwrap().models.is_empty());
    let retry = request(key(&mut panel, KeyCode::Enter));
    panel.complete(Reply {
        id: first.id,
        result: Ok((choices(), Some(Ok(vec!["stale".into()])))),
    });
    assert!(panel.form().unwrap().models.is_empty());
    assert!(panel.pending.is_some());
    panel.complete(Reply {
        id: retry.id,
        result: Ok((choices(), Some(Ok(vec!["new-model".into()])))),
    });
    assert_eq!(panel.form().unwrap().models, vec!["new-model"]);
    let empty = request(key(&mut panel, KeyCode::Enter));
    panel.complete(Reply {
        id: empty.id,
        result: Ok((choices(), Some(Ok(Vec::new())))),
    });
    assert!(panel.form().unwrap().models.is_empty());
    assert!(
        panel
            .form()
            .unwrap()
            .message
            .contains("Provider returned no models")
    );
}
