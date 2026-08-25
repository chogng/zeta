use zeta_ui::ScrollCommand;
use zeta_ui::ScrollDelta;
use zui::input::MouseScrollDelta;
use zui::ui::ElementId;

use crate::remote_connection_manager::REMOTE_CONNECTION_MANAGER;
use crate::remote_connection_manager::REMOTE_CONNECTION_MANAGER_CLOSE;
use crate::remote_connection_manager::REMOTE_CONNECTION_MANAGER_CONNECT;
use crate::remote_connection_manager::REMOTE_CONNECTION_MANAGER_DELETE;
use crate::remote_connection_manager::REMOTE_CONNECTION_MANAGER_HOST;
use crate::remote_connection_manager::REMOTE_CONNECTION_MANAGER_ITEM_HEIGHT;
use crate::remote_connection_manager::REMOTE_CONNECTION_MANAGER_LIST;
use crate::remote_connection_manager::REMOTE_CONNECTION_MANAGER_NAME;
use crate::remote_connection_manager::REMOTE_CONNECTION_MANAGER_NEW;
use crate::remote_connection_manager::REMOTE_CONNECTION_MANAGER_SAVE;
use crate::remote_connection_manager::REMOTE_CONNECTION_MANAGER_STATUS;
use crate::remote_connection_manager::REMOTE_CONNECTION_MANAGER_WORKSPACE;
use crate::remote_connection_manager::remote_connection_manager_item_index;

const MANAGER_ROWS_PER_WHEEL_STEP: f32 = 3.0;

pub(super) fn remote_connection_manager_scroll_command(delta: MouseScrollDelta) -> ScrollCommand {
    let pixels = match delta {
        MouseScrollDelta::LineDelta(_, vertical) => {
            vertical * MANAGER_ROWS_PER_WHEEL_STEP * REMOTE_CONNECTION_MANAGER_ITEM_HEIGHT
        }
        MouseScrollDelta::PixelDelta(position) => position.y as f32,
    };
    ScrollCommand::ByPixels(ScrollDelta::vertical(-pixels))
}

pub(super) fn is_remote_connection_manager_element(id: ElementId, item_count: usize) -> bool {
    matches!(
        id,
        REMOTE_CONNECTION_MANAGER
            | REMOTE_CONNECTION_MANAGER_CLOSE
            | REMOTE_CONNECTION_MANAGER_NEW
            | REMOTE_CONNECTION_MANAGER_NAME
            | REMOTE_CONNECTION_MANAGER_HOST
            | REMOTE_CONNECTION_MANAGER_WORKSPACE
            | REMOTE_CONNECTION_MANAGER_SAVE
            | REMOTE_CONNECTION_MANAGER_DELETE
            | REMOTE_CONNECTION_MANAGER_CONNECT
            | REMOTE_CONNECTION_MANAGER_LIST
            | REMOTE_CONNECTION_MANAGER_STATUS
    ) || remote_connection_manager_item_index(id, item_count).is_some()
}
