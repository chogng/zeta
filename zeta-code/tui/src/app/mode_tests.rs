use super::App;
use super::AppEvent;
use crate::config::Event as ConfigEvent;
use crate::config::TerminalSettings;
use crate::terminal::ScreenMode;
use crate::thread::Event as ThreadEvent;
use crate::thread::composer::ChatInputQueueOutcome;
use crate::widgets::list_selection::ListSelectionGroup;
use crate::widgets::list_selection::ListSelectionItem;
use crate::widgets::list_selection::ListSelectionModel;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::layout::Position;
use ratatui::layout::Rect;

fn switch(app: &mut App, mode: ScreenMode) {
    let mut settings = TerminalSettings::default();
    settings.set_screen_mode(mode);
    app.update(ConfigEvent::SettingsReceived(settings));
}

fn render(app: &App) -> String {
    let mut terminal = Terminal::new(TestBackend::new(50, 16)).unwrap();
    terminal
        .draw(|frame| super::frame::draw(frame, app))
        .unwrap();
    terminal
        .backend()
        .buffer()
        .content
        .chunks(50)
        .map(|row| row.iter().map(|cell| cell.symbol()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn modes_restore_their_own_scroll_while_sharing_the_draft_and_queue() {
    let mut app = App::new();
    for index in 0..16 {
        app.update(ThreadEvent::FailureReported(format!("message {index:02}")));
    }
    app.insert_text("queued message");
    let ChatInputQueueOutcome::Queued(queued) =
        app.thread_presentations.active_mut().input.queue_current()
    else {
        panic!("the draft must become a queue entry");
    };
    app.thread_presentations.active_mut().queue.push(queued);
    app.insert_text("shared draft");
    let area = Rect::new(0, 0, 50, 16);
    app.handle_key_in_area(KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE), area);
    let fullscreen_anchor = app.transcript_scroll().anchor().cloned();
    assert!(fullscreen_anchor.is_some());
    insta::assert_snapshot!("fullscreen_scroll_with_shared_draft", render(&app));

    switch(&mut app, ScreenMode::Inline);
    assert!(app.transcript_scroll().anchor().is_none());
    assert_eq!(app.input(), "shared draft");
    assert_eq!(app.queue_view().items[0].text, "queued message");
    app.handle_key_in_area(KeyEvent::new(KeyCode::Home, KeyModifiers::CONTROL), area);
    let inline_anchor = app.transcript_scroll().anchor().cloned();
    assert!(inline_anchor.is_some());
    assert_ne!(inline_anchor, fullscreen_anchor);
    insta::assert_snapshot!("inline_scroll_with_shared_draft", render(&app));

    switch(&mut app, ScreenMode::Fullscreen);
    assert_eq!(app.transcript_scroll().anchor(), fullscreen_anchor.as_ref());
    switch(&mut app, ScreenMode::Inline);
    assert_eq!(app.transcript_scroll().anchor(), inline_anchor.as_ref());
    assert_eq!(app.input(), "shared draft");
    assert_eq!(app.queue_view().items[0].text, "queued message");
}

#[test]
fn switching_modes_moves_the_active_panel_and_keeps_its_keyboard_selection() {
    let mut app = App::new();
    app.insert_text("keep editing this draft");
    app.update(AppEvent::HelpOpened(ListSelectionModel::new(
        "Help",
        vec![ListSelectionGroup::new(
            "Commands",
            vec![
                ListSelectionItem::new("First"),
                ListSelectionItem::new("Second"),
            ],
        )],
    )));
    app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    assert_eq!(
        app.list_selection().unwrap().selected_visible_index(),
        Some(1)
    );

    switch(&mut app, ScreenMode::Inline);
    assert!(app.fullscreen.panels.command().is_none());
    assert!(app.inline.panels.command().is_some());
    assert_eq!(
        app.list_selection().unwrap().selected_visible_index(),
        Some(1)
    );
    assert!(!app.chat_input_focused());
    insta::assert_snapshot!("panel_transferred_to_inline", render(&app));

    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(app.command_panel().is_none());
    assert!(app.chat_input_focused());
    assert_eq!(app.input(), "keep editing this draft");
    insta::assert_snapshot!("inline_draft_after_panel_dismissed", render(&app));
    switch(&mut app, ScreenMode::Fullscreen);
    assert!(app.command_panel().is_none());
    assert_eq!(app.input(), "keep editing this draft");
}

#[test]
fn clearing_the_conversation_removes_selection_from_the_inactive_mode() {
    let mut app = App::new();
    app.update(ThreadEvent::ContextChanged {
        session_id: zeta_protocol::SessionId::new("session").unwrap(),
        thread_id: zeta_protocol::ThreadId::new("thread").unwrap(),
    });
    app.update(ThreadEvent::FailureReported("message to clear".into()));
    app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::CONTROL));
    assert!(app.transcript_selection_active());
    switch(&mut app, ScreenMode::Inline);
    assert!(!app.transcript_selection_active());
    app.update(ThreadEvent::TranscriptCleared);
    switch(&mut app, ScreenMode::Fullscreen);
    assert!(!app.transcript_selection_active());
    assert!(app.transcript_scroll().anchor().is_none());
    assert!(app.chat_input_focused());
}

fn focus_app() -> App {
    let mut app = App::new();
    app.update(ThreadEvent::ContextChanged {
        session_id: zeta_protocol::SessionId::new("focus-session").unwrap(),
        thread_id: zeta_protocol::ThreadId::new("focus-thread").unwrap(),
    });
    for index in 0..8 {
        app.update(ThreadEvent::FailureReported(format!(
            "focus message {index:02}"
        )));
    }
    app
}

#[test]
fn switching_modes_releases_queue_focus_before_restoring_transcript_focus() {
    for (target, other, snapshot) in [
        (
            ScreenMode::Fullscreen,
            ScreenMode::Inline,
            "restored_fullscreen_transcript_focus",
        ),
        (
            ScreenMode::Inline,
            ScreenMode::Fullscreen,
            "restored_inline_transcript_focus",
        ),
    ] {
        let mut app = focus_app();
        app.insert_text("queued message");
        let ChatInputQueueOutcome::Queued(queued) =
            app.thread_presentations.active_mut().input.queue_current()
        else {
            panic!("queued input expected");
        };
        app.thread_presentations.active_mut().queue.push(queued);
        switch(&mut app, target);
        app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::CONTROL));
        assert!(app.transcript_selection_active());

        switch(&mut app, other);
        app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::ALT));
        assert!(app.queue_focused());
        // Reloading the same setting must not end an ongoing interaction.
        switch(&mut app, other);
        assert!(app.queue_focused());

        switch(&mut app, target);
        assert!(!app.queue_focused());
        assert!(app.transcript_selection_active());
        assert_eq!(app.queue_view().items[0].text, "queued message");
        insta::assert_snapshot!(snapshot, render(&app));

        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(!app.transcript_selection_active());
        assert!(app.chat_input_focused());
    }
}

#[test]
fn escape_returns_to_the_shared_draft_after_restoring_transcript_selection() {
    for (target, other, snapshot) in [
        (
            ScreenMode::Fullscreen,
            ScreenMode::Inline,
            "fullscreen_draft_after_selection_escape",
        ),
        (
            ScreenMode::Inline,
            ScreenMode::Fullscreen,
            "inline_draft_after_selection_escape",
        ),
    ] {
        let mut app = focus_app();
        switch(&mut app, target);
        app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::CONTROL));
        assert!(app.transcript_selection_active());
        switch(&mut app, other);
        app.handle_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
        assert_eq!(app.input(), "x");
        assert!(app.chat_input_focused());

        switch(&mut app, target);
        assert!(app.transcript_selection_active());
        for kind in [KeyEventKind::Release, KeyEventKind::Repeat] {
            app.handle_key(KeyEvent::new_with_kind(
                KeyCode::Esc,
                KeyModifiers::NONE,
                kind,
            ));
            assert!(app.transcript_selection_active());
        }
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(app.input(), "x");
        assert!(!app.transcript_selection_active());
        assert!(app.chat_input_focused());
        insta::assert_snapshot!(snapshot, render(&app));

        let area = Rect::new(0, 0, 50, 16);
        let input = match target {
            ScreenMode::Fullscreen => super::fullscreen::layout(&app, area).input,
            ScreenMode::Inline => super::inline::layout(&app, area).input,
        };
        let mut terminal = Terminal::new(TestBackend::new(area.width, area.height)).unwrap();
        terminal
            .draw(|frame| super::frame::draw(frame, &app))
            .unwrap();
        assert_eq!(
            terminal.get_cursor_position().unwrap(),
            Position::new(input.x + 3, input.y + 1)
        );
    }
}

#[test]
fn switching_modes_returns_manager_focus_to_input_without_activating_hidden_selection() {
    for (target, other) in [
        (ScreenMode::Fullscreen, ScreenMode::Inline),
        (ScreenMode::Inline, ScreenMode::Fullscreen),
    ] {
        let mut app = focus_app();
        switch(&mut app, target);
        app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::CONTROL));
        assert!(app.transcript_selection_active());
        switch(&mut app, other);
        app.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        assert!(app.session_manager_focused());
        switch(&mut app, target);
        assert!(app.session_manager_view().is_some());
        assert!(!app.session_manager_focused());
        assert!(!app.transcript_selection_active());
        assert!(app.chat_input_focused());
    }
}
