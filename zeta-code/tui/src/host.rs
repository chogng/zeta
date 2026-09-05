pub(crate) mod browser;
pub(crate) mod clipboard;
pub(crate) mod process_resources;
mod termination;
pub(crate) mod transcript_export;

/// A completed host operation delivered to the TUI state owner.
pub(crate) enum Event {
    ClipboardImageRead(Result<clipboard::ClipboardImage, String>),
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

pub(crate) enum Operation {
    CopyLastResponse(Result<String, String>),
    ExportTranscript {
        root: std::path::PathBuf,
        requested_path: Option<std::path::PathBuf>,
        markdown: String,
    },
    ReadClipboardImage,
    RefreshClipboardImageAvailability,
}

impl Operation {
    pub(crate) const fn name(&self) -> &'static str {
        match self {
            Self::CopyLastResponse(_) => "zeta-tui-copy-last-response",
            Self::ExportTranscript { .. } => "zeta-tui-export-transcript",
            Self::ReadClipboardImage => "zeta-tui-read-clipboard-image",
            Self::RefreshClipboardImageAvailability => {
                "zeta-tui-refresh-clipboard-image-availability"
            }
        }
    }

    pub(crate) fn execute(self) -> Event {
        match self {
            Self::CopyLastResponse(response) => copy_last_response(response),
            Self::ExportTranscript {
                root,
                requested_path,
                markdown,
            } => export_transcript(root, requested_path, markdown),
            Self::ReadClipboardImage => Event::ClipboardImageRead(clipboard::read_image()),
            Self::RefreshClipboardImageAvailability => {
                Event::ClipboardImageAvailabilityChanged(clipboard::image_availability())
            }
        }
    }
}

fn copy_last_response(response: Result<String, String>) -> Event {
    match response.and_then(|response| {
        let char_count = response.chars().count();
        clipboard::write_text(&response).map(|()| char_count)
    }) {
        Ok(char_count) => {
            Event::TopTipNoticeShown(format!("Copied {char_count} chars to clipboard"))
        }
        Err(error) => Event::OperationCompleted(Err(error)),
    }
}

fn export_transcript(
    root: std::path::PathBuf,
    requested_path: Option<std::path::PathBuf>,
    markdown: String,
) -> Event {
    let result = if markdown.is_empty() {
        Err("there is no conversation to export".to_owned())
    } else {
        transcript_export::write(&root, requested_path.as_deref(), &markdown)
            .map(|path| format!("Exported conversation to {}", path.display()))
    };
    Event::OperationCompleted(result)
}

pub(crate) use termination::TerminationSource;
