use super::ThreadPresentationStore;
use crate::components::chat_input::ChatInputQueueOutcome;
use crate::features::thread::TranscriptCellId;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use zeta_protocol::ThreadId;

#[test]
fn switching_threads_restores_draft_queue_and_scroll_together() {
    let main = thread_id("main");
    let child = thread_id("child");
    let mut store = ThreadPresentationStore::new(main.clone());
    store.active_mut().input.insert_text("main draft");
    let ChatInputQueueOutcome::Queued(queued) = store.active_mut().input.queue_current(None) else {
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

fn thread_id(value: &str) -> ThreadId {
    ThreadId::new(value).unwrap()
}
