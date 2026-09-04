pub(crate) mod browser;
pub(crate) mod clipboard;
pub(crate) mod process_resources;
mod termination;
pub(crate) mod transcript_export;

/// A completed host operation delivered to the TUI state owner.
pub(crate) enum Event {
    ClipboardImageRead(Result<Vec<u8>, String>),
    ClipboardImageAvailabilityChanged(clipboard::ClipboardImageAvailability),
    OperationCompleted(Result<String, String>),
    ProcessResourcesSampled(process_resources::ProcessResourcesReading),
    TopTipNoticeShown(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Command {
    CopyLastResponse,
    ExportTranscript {
        requested_path: Option<std::path::PathBuf>,
    },
    ReadClipboardImage,
    RefreshClipboardImageAvailability,
}

pub(crate) use termination::TerminationSource;
