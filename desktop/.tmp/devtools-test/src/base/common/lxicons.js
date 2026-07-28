import { lxAdd, lxChat, lxChatFilled, lxChevronDown, lxChevronRight, lxGear, lxLayoutSidebarLeft, lxLayoutSidebarLeftOff, lxMenu, lxStart, } from "@chogng/lxicons";
import { registerIcon } from "./icon.js";
/**
 * Default icons supplied by Lxicons.
 *
 * Keep vendor imports in this library instead of exposing SVG factories to
 * controls and product code. Add entries here only as the application uses
 * them so the renderer bundle remains tree-shakable.
 */
export const LxIcon = {
    add: registerIcon("add", lxAdd),
    chat: registerIcon("chat", lxChat),
    chatFilled: registerIcon("chat-filled", lxChatFilled),
    dropdownIndicator: registerIcon("dropdown-indicator", lxChevronDown),
    gear: registerIcon("gear", lxGear),
    layoutSidebarLeft: registerIcon("layout-sidebar-left", lxLayoutSidebarLeft),
    layoutSidebarLeftOff: registerIcon("layout-sidebar-left-off", lxLayoutSidebarLeftOff),
    menu: registerIcon("menu", lxMenu),
    start: registerIcon("start", lxStart),
    submenuIndicator: registerIcon("submenu-indicator", lxChevronRight),
};
