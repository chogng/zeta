use super::MAX_THREAD_PRESENTATIONS;
use super::ThreadPresentationStore;
use crate::thread::TranscriptCellId;
use crate::thread::composer::ChatInputCatalog;
use crate::thread::composer::ChatInputQueueOutcome;
use crate::thread::composer::CompletionView;
use crate::thread::composer::built_in_slash_command_definitions;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use zeta_protocol::ThreadId;
use zeta_slash_commands::SlashCommandArgumentMode;
use zeta_slash_commands::SlashCommandCatalog;
use zeta_slash_commands::SlashCommandDefinition;

#[test]
fn switching_threads_restores_draft_queue_and_scroll_together() {
    let main = thread_id("main");
    let child = thread_id("child");
    let mut store = ThreadPresentationStore::new(main.clone());
    store.active_mut().input.insert_text("main draft");
    let ChatInputQueueOutcome::Queued(queued) = store.active_mut().input.queue_current() else {
        panic!("expected queued input");
    };
    store.active_mut().queue.push(queued);
    store.active_mut().input.insert_text("main remaining draft");
    store
        .active_mut()
        .scroll
        .handle_key(KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE));

    store.switch(child);
    store.active_mut().input.insert_text("child draft");
    assert_eq!(store.active().scroll.paragraph_offset(20), 20);
    store.switch(main);

    assert_eq!(store.active().input.text(), "main remaining draft");
    assert_eq!(store.active().queue.view().items[0].text, "main draft");
    assert_eq!(store.active().scroll.paragraph_offset(20), 15);
}

#[test]
fn switching_threads_restores_cell_selection_and_expansion_together() {
    let main = thread_id("main");
    let child = thread_id("child");
    let mut store = ThreadPresentationStore::new(main.clone());
    let main_cell = TranscriptCellId::from_render_key("entry:main-entry");
    let child_cell = TranscriptCellId::from_render_key("entry:child-entry");
    assert!(store.active_mut().toggle_cell(&main_cell));

    store.switch(child);
    assert!(store.active().selected_cell.is_none());
    assert!(store.active().expanded_cells.is_empty());
    assert!(
        store
            .active_mut()
            .select_next_cell(std::slice::from_ref(&child_cell))
    );

    store.switch(main);
    assert_eq!(store.active().selected_cell.as_ref(), Some(&main_cell));
    assert!(store.active().expanded_cells.contains(&main_cell));
}

#[test]
fn refreshed_completion_catalog_reaches_existing_and_future_threads() {
    let main = thread_id("main");
    let child = thread_id("child");
    let future = thread_id("future");
    let mut store = ThreadPresentationStore::new(main.clone());
    store.switch(child.clone());
    store.switch(main.clone());
    let catalog = SlashCommandCatalog::with_local_and_server(
        built_in_slash_command_definitions(),
        [SlashCommandDefinition {
            name: "diagnose".into(),
            description: "inspect the current dir".into(),
            argument_mode: SlashCommandArgumentMode::Optional,
        }],
    )
    .unwrap();

    store.replace_input_catalog(ChatInputCatalog::with_slash_commands(catalog));

    for thread in [main, child, future] {
        store.switch(thread);
        store.active_mut().input.insert_text("/diag");
        let Some(CompletionView::Slash(view)) = store.active().input.completion() else {
            panic!("expected Slash completion");
        };
        assert_eq!(view.commands[0].name, "diagnose");
    }
}

#[test]
fn thread_presentations_evict_the_least_recent_inactive_thread() {
    let first = thread_id("thread-0");
    let retained = thread_id("thread-1");
    let mut store = ThreadPresentationStore::new(first.clone());

    for index in 1..MAX_THREAD_PRESENTATIONS {
        store.switch(thread_id(&format!("thread-{index}")));
    }
    store.switch(retained.clone());
    store.switch(thread_id(&format!("thread-{MAX_THREAD_PRESENTATIONS}")));

    assert_eq!(store.len(), MAX_THREAD_PRESENTATIONS);
    assert!(!store.contains(&first));
    assert!(store.contains(&retained));
    assert!(store.contains(&thread_id(&format!("thread-{MAX_THREAD_PRESENTATIONS}"))));
}

fn thread_id(value: &str) -> ThreadId {
    ThreadId::new(value).unwrap()
}
