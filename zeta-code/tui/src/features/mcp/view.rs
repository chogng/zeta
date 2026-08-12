use crate::components::pane::PaneViewModel;
use crate::components::search_box::SearchBoxModel;
use crate::components::selection::SelectionActivationMode;
use crate::components::selection::SelectionItem;
use crate::components::selection::SelectionItemId;
use crate::components::selection::SelectionTab;
use crate::components::selection::SelectionViewModel;
use std::collections::BTreeMap;
use zeta_app_server_protocol::protocol::config::McpServerConfigDto;
use zeta_app_server_protocol::protocol::config::McpServerEnablementDto;
use zeta_app_server_protocol::protocol::config::McpTransportDto;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum McpSelectionAction {
    SetEnablement {
        server_id: String,
        enablement: McpServerEnablementDto,
    },
}

pub(crate) struct McpSelectionView {
    pub(crate) model: PaneViewModel<SelectionViewModel>,
    pub(crate) actions: BTreeMap<SelectionItemId, McpSelectionAction>,
}

pub(crate) fn mcp_selection_view(
    servers: &BTreeMap<String, McpServerConfigDto>,
) -> McpSelectionView {
    let mut actions = BTreeMap::new();
    let all = servers
        .values()
        .enumerate()
        .map(|(index, server)| mcp_item(index, server, &mut actions))
        .collect::<Vec<_>>();
    let enabled = all
        .iter()
        .zip(servers.values())
        .filter(|(_, server)| server.enablement == McpServerEnablementDto::Enabled)
        .map(|(item, _)| item.clone())
        .collect::<Vec<_>>();
    let disabled = all
        .iter()
        .zip(servers.values())
        .filter(|(_, server)| server.enablement == McpServerEnablementDto::Disabled)
        .map(|(item, _)| item.clone())
        .collect::<Vec<_>>();
    let enabled_count = enabled.len();
    let disabled_count = disabled.len();

    McpSelectionView {
        model: PaneViewModel::new(
            SelectionViewModel::new(
                "MCP servers",
                vec![
                    SelectionTab::new(format!("All ({})", all.len()), all),
                    SelectionTab::new(format!("Enabled ({enabled_count})"), enabled),
                    SelectionTab::new(format!("Disabled ({disabled_count})"), disabled),
                ],
            )
            .with_activation_mode(SelectionActivationMode::Enter)
            .with_search(SearchBoxModel::new("Search MCP servers"))
            .with_empty_message("No matching MCP servers"),
            "Space search  ·  ←/→ tabs  ·  ↑/↓ select  ·  Enter toggle  ·  Esc back",
        ),
        actions,
    }
}

fn mcp_item(
    index: usize,
    server: &McpServerConfigDto,
    actions: &mut BTreeMap<SelectionItemId, McpSelectionAction>,
) -> SelectionItem {
    let item_id = SelectionItemId::new(format!("mcp-{index}"));
    let next_enablement = match server.enablement {
        McpServerEnablementDto::Disabled => McpServerEnablementDto::Enabled,
        McpServerEnablementDto::Enabled => McpServerEnablementDto::Disabled,
    };
    actions.insert(
        item_id.clone(),
        McpSelectionAction::SetEnablement {
            server_id: server.id.clone(),
            enablement: next_enablement,
        },
    );
    SelectionItem::new(&server.display_name)
        .with_id(item_id)
        .with_description(format!(
            "{}  ·  {}  ·  {}",
            server.id,
            enablement_label(server.enablement),
            transport_label(&server.transport)
        ))
}

fn enablement_label(enablement: McpServerEnablementDto) -> &'static str {
    match enablement {
        McpServerEnablementDto::Disabled => "disabled",
        McpServerEnablementDto::Enabled => "enabled",
    }
}

fn transport_label(transport: &McpTransportDto) -> String {
    match transport {
        McpTransportDto::Stdio { command, args } => std::iter::once(command.as_str())
            .chain(args.iter().map(String::as_str))
            .collect::<Vec<_>>()
            .join(" "),
        McpTransportDto::StreamableHttp { url } => url.clone(),
    }
}

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;
