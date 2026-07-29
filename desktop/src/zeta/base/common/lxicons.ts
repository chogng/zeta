import { lxAdd, lxBrowserWeb, lxChat, lxChatFilled, lxChevronDown, lxChevronRight, lxClose, lxEllipsis, lxFiles, lxGear, lxGitCommit, lxLayoutPanel, lxLayoutPanelOff, lxLayoutSidebarLeft, lxLayoutSidebarLeftOff, lxLayoutSidebarRight, lxLayoutSidebarRightOff, lxLinkExternal, lxMenu, lxRefresh, lxSearch, lxSettings, lxSplitHorizontal, lxStart } from "@chogng/lxicons";
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
  browserWeb: registerIcon("browser-web", lxBrowserWeb),
  chat: registerIcon("chat", lxChat),
  chatFilled: registerIcon("chat-filled", lxChatFilled),
  close: registerIcon("close", lxClose),
  dropdownIndicator: registerIcon("dropdown-indicator", lxChevronDown),
  ellipsis: registerIcon("ellipsis", lxEllipsis),
  files: registerIcon("files", lxFiles),
  gear: registerIcon("gear", lxGear),
  gitCommit: registerIcon("git-commit", lxGitCommit),
  history: registerIcon("history", lxRefresh),
  layoutSidebarLeft: registerIcon(
    "layout-sidebar-left",
    lxLayoutSidebarLeft,
  ),
  layoutSidebarLeftOff: registerIcon(
    "layout-sidebar-left-off",
    lxLayoutSidebarLeftOff,
  ),
  layoutSidebarRight: registerIcon(
    "layout-sidebar-right",
    lxLayoutSidebarRight,
  ),
  layoutSidebarRightOff: registerIcon(
    "layout-sidebar-right-off",
    lxLayoutSidebarRightOff,
  ),
  layoutPanel: registerIcon("layout-panel", lxLayoutPanel),
  layoutPanelOff: registerIcon("layout-panel-off", lxLayoutPanelOff),
  linkExternal: registerIcon("link-external", lxLinkExternal),
  menu: registerIcon("menu", lxMenu),
  search: registerIcon("search", lxSearch),
  settings: registerIcon("settings", lxSettings),
  splitHorizontal: registerIcon("split-horizontal", lxSplitHorizontal),
  start: registerIcon("start", lxStart),
  submenuIndicator: registerIcon("submenu-indicator", lxChevronRight),
} as const;
