use crate::components::pane::PaneViewModel;
use crate::components::search_box::SearchBoxModel;
use crate::components::selection::SelectionItem;
use crate::components::selection::SelectionItemId;
use crate::components::selection::SelectionPreview;
use crate::components::selection::SelectionTab;
use crate::components::selection::SelectionViewModel;
use ratatui::text::Line;
use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;
use zeta_app_server_client::AppServerClient;
use zeta_app_server_client::ClientError;
use zeta_app_server_client::JsonRpcTransport;
use zeta_app_server_protocol::protocol::fs::FsFileType;
use zeta_app_server_protocol::protocol::fs::FsReadDirectoryEntry;
use zeta_app_server_protocol::protocol::fs::FsReadDirectoryParams;
use zeta_app_server_protocol::protocol::fs::FsReadFileParams;

const MAX_PREVIEW_BYTES: usize = 64 * 1024;
const MAX_PREVIEW_LINES: usize = 200;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum FileSelectionAction {
    OpenDirectory { path: PathBuf },
    PreviewFile { path: PathBuf },
}

pub(crate) struct FileSelectionView {
    pub(crate) model: PaneViewModel<SelectionViewModel>,
    pub(crate) actions: BTreeMap<SelectionItemId, FileSelectionAction>,
}

pub(crate) fn load_directory<T>(
    client: &mut AppServerClient<T>,
    path: PathBuf,
) -> Result<FileSelectionView, ClientError>
where
    T: JsonRpcTransport,
{
    client
        .read_directory(FsReadDirectoryParams { path: path.clone() })
        .map(|result| directory_view(path, &result.entries))
}

pub(crate) fn load_file_preview<T>(
    client: &mut AppServerClient<T>,
    path: PathBuf,
) -> Result<PaneViewModel<SelectionViewModel>, ClientError>
where
    T: JsonRpcTransport,
{
    client
        .read_file(FsReadFileParams { path: path.clone() })
        .map(|result| file_preview(path, &result.content, &result.revision))
}

fn directory_view(path: PathBuf, entries: &[FsReadDirectoryEntry]) -> FileSelectionView {
    let mut actions = BTreeMap::new();
    let mut items = Vec::new();
    if !path.as_os_str().is_empty() {
        let parent = path.parent().unwrap_or_else(|| Path::new("")).to_path_buf();
        let item_id = SelectionItemId::new("files-parent");
        actions.insert(
            item_id.clone(),
            FileSelectionAction::OpenDirectory { path: parent },
        );
        items.push(
            SelectionItem::new("../")
                .with_id(item_id)
                .with_description("parent directory"),
        );
    }
    let mut sorted = entries.to_vec();
    sorted.sort_by_key(|entry| {
        (
            !matches!(entry.file_type, FsFileType::Directory),
            entry.name.to_lowercase(),
        )
    });
    for (index, entry) in sorted.iter().enumerate() {
        let child = path.join(&entry.name);
        let item_id = SelectionItemId::new(format!("files-entry-{index}"));
        let action = match entry.file_type {
            FsFileType::Directory => Some(FileSelectionAction::OpenDirectory {
                path: child.clone(),
            }),
            FsFileType::File => Some(FileSelectionAction::PreviewFile {
                path: child.clone(),
            }),
            FsFileType::SymbolicLink | FsFileType::Other => None,
        };
        if let Some(action) = action {
            actions.insert(item_id.clone(), action);
        }
        items.push(
            SelectionItem::new(format!(
                "{}{}",
                entry.name,
                if entry.file_type == FsFileType::Directory {
                    "/"
                } else {
                    ""
                }
            ))
            .with_id(item_id)
            .with_description(file_type_label(entry.file_type)),
        );
    }
    FileSelectionView {
        model: PaneViewModel::new(
            SelectionViewModel::new(
                if path.as_os_str().is_empty() {
                    "Workspace files".into()
                } else {
                    format!("Files · {}", path.display())
                },
                vec![SelectionTab::new("Entries", items)],
            )
            .without_tab_bar()
            .with_search(SearchBoxModel::new("Search this directory"))
            .with_empty_message("This directory is empty"),
            "Space search  ·  ↑/↓ select  ·  Enter open  ·  Esc back",
        ),
        actions,
    }
}

fn file_preview(path: PathBuf, content: &str, revision: &str) -> PaneViewModel<SelectionViewModel> {
    let mut bytes = 0usize;
    let mut truncated = false;
    let mut lines = Vec::new();
    for line in content.lines() {
        if lines.len() >= MAX_PREVIEW_LINES || bytes.saturating_add(line.len()) > MAX_PREVIEW_BYTES
        {
            truncated = true;
            break;
        }
        bytes = bytes.saturating_add(line.len());
        lines.push(Line::from(line.to_owned()));
    }
    if truncated {
        lines.push(Line::from("… preview truncated …"));
    }
    if lines.is_empty() {
        lines.push(Line::from("(empty file)"));
    }
    let preview = SelectionPreview::new("UTF-8 preview", lines).with_margins(1, 0);
    PaneViewModel::new(
        SelectionViewModel::new(
            format!("File · {}", path.display()),
            vec![SelectionTab::new(
                "Preview",
                vec![
                    SelectionItem::new(path.display().to_string())
                        .with_description(format!("revision {}", short_revision(revision)))
                        .with_preview(preview),
                ],
            )],
        )
        .without_tab_bar()
        .without_selection(),
        "Esc back",
    )
}

fn file_type_label(file_type: FsFileType) -> &'static str {
    match file_type {
        FsFileType::Directory => "directory",
        FsFileType::File => "file",
        FsFileType::SymbolicLink => "symbolic link",
        FsFileType::Other => "other",
    }
}

fn short_revision(revision: &str) -> &str {
    revision.get(..revision.len().min(12)).unwrap_or(revision)
}

#[cfg(test)]
#[path = "browser_tests.rs"]
mod tests;
