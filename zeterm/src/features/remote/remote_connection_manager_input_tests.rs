use zui::ui::ElementId;

use super::is_remote_connection_manager_element;
use crate::remote_connection_manager::REMOTE_CONNECTION_MANAGER_NAME;
use crate::remote_connection_manager::remote_connection_manager_item_id;
use crate::shell_interaction::COMPOSER;

#[test]
fn modal_pointer_gate_accepts_only_manager_roots_controls_and_live_items() {
    assert!(is_remote_connection_manager_element(
        REMOTE_CONNECTION_MANAGER_NAME,
        2
    ));
    assert!(is_remote_connection_manager_element(
        remote_connection_manager_item_id(1),
        2
    ));
    assert!(!is_remote_connection_manager_element(
        remote_connection_manager_item_id(2),
        2
    ));
    assert!(!is_remote_connection_manager_element(COMPOSER, 2));
    assert!(!is_remote_connection_manager_element(
        ElementId::scoped(99, 99),
        2
    ));
}
