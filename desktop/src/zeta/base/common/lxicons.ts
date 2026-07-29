import {
  lxAdd,
  lxChat,
  lxChatFilled,
  lxChevronDown,
  lxChevronRight,
  lxClose,
  lxFiles,
  lxGear,
  lxGitCommit,
  lxLayoutPanel,
  lxLayoutPanelOff,
  lxLayoutSidebarLeft,
  lxLayoutSidebarLeftOff,
  lxMenu,
  lxSearch,
  lxStart,
} from "@chogng/lxicons";
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
  close: registerIcon("close", lxClose),
  dropdownIndicator: registerIcon("dropdown-indicator", lxChevronDown),
  files: registerIcon("files", lxFiles),
  gear: registerIcon("gear", lxGear),
  gitCommit: registerIcon("git-commit", lxGitCommit),
  layoutSidebarLeft: registerIcon(
    "layout-sidebar-left",
    lxLayoutSidebarLeft,
  ),
  layoutSidebarLeftOff: registerIcon(
    "layout-sidebar-left-off",
    lxLayoutSidebarLeftOff,
  ),
  layoutPanel: registerIcon("layout-panel", lxLayoutPanel),
  layoutPanelOff: registerIcon("layout-panel-off", lxLayoutPanelOff),
  menu: registerIcon("menu", lxMenu),
  search: registerIcon("search", lxSearch),
  start: registerIcon("start", lxStart),
  submenuIndicator: registerIcon("submenu-indicator", lxChevronRight),
} as const;
