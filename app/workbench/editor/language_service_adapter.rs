//! Product event-loop adapter for editor-owned language-service events.

use zeta_editor_host::FileEditorLanguageEvent;
use zeta_editor_host::FileEditorLanguageEventSink;
use zui::app::AppProxy;

use crate::product_event::ProductEvent;

pub(crate) struct ProductLanguageEventSink {
    events: AppProxy<ProductEvent>,
}

impl ProductLanguageEventSink {
    pub(crate) const fn new(events: AppProxy<ProductEvent>) -> Self {
        Self { events }
    }
}

impl FileEditorLanguageEventSink for ProductLanguageEventSink {
    fn send(&self, event: FileEditorLanguageEvent) -> std::result::Result<(), String> {
        self.events
            .send_event(ProductEvent::EditorLanguage(event))
            .map_err(|_| "desktop event loop is unavailable".to_owned())
    }
}
