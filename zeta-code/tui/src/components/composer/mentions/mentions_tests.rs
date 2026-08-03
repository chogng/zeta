use super::Mentions;
use super::input::active_mention;
use super::popup::MentionPopup;
use crate::components::composer::editor::TextArea;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use std::path::PathBuf;
use zeta_file_search::PathMatch;
use zeta_file_search::PathSearchSnapshot;

#[test]
fn active_token_resolves_at_the_cursor_without_matching_email_text() {
    assert_eq!(
        active_mention("review @src/lib", "review @src".len()),
        Some(super::input::ActiveMention {
            range: 7..15,
            query: "src/lib",
        })
    );
    assert_eq!(active_mention("mail@example.com", 8), None);
}

#[test]
fn popup_ignores_stale_results_and_preserves_dismissal_for_an_unchanged_token() {
    let mut popup = MentionPopup::default();
    let active = active_mention("@src", 4);
    popup.sync(active.clone());
    popup.apply_search_snapshot(snapshot("old", &["old.rs"]));
    assert!(popup.view().unwrap().matches.is_empty());
    popup.apply_search_snapshot(snapshot("src", &["src/lib.rs", "src/main.rs"]));

    popup.select_previous();
    assert_eq!(popup.view().unwrap().selected, 1);
    popup.select_next();
    assert_eq!(popup.view().unwrap().selected, 0);

    popup.dismiss();
    popup.sync(active);
    assert_eq!(popup.view(), None);
}

#[test]
fn completing_a_file_replaces_only_the_token_with_an_atomic_path() {
    let mut mentions = Mentions {
        popup: MentionPopup::default(),
    };
    let mut textarea = TextArea::new();
    textarea.insert_text("review @src please");
    for _ in 0.." please".len() {
        textarea.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
    }
    mentions.sync_textarea(&textarea);
    mentions.apply_search_snapshot(snapshot("src", &["src/lib.rs"]));

    assert!(mentions.complete_selected(&mut textarea));
    assert_eq!(textarea.text(), "review src/lib.rs please");
    let (_, range) = textarea.elements().next().unwrap();
    assert_eq!(&textarea.text()[range], "src/lib.rs");
}

fn snapshot(query: &str, paths: &[&str]) -> PathSearchSnapshot {
    PathSearchSnapshot {
        query: query.to_owned(),
        matches: paths
            .iter()
            .map(|path| PathMatch {
                score: 1,
                path: PathBuf::from(path),
                indices: Vec::new(),
            })
            .collect(),
        search_complete: true,
        ..PathSearchSnapshot::default()
    }
}
