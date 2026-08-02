import * as svg from "../../../../generated/product-icons.js";
import { register } from "./icon.js";

/**
 * Lxicons supplied by Zeta's repository-owned product resources.
 *
 * Keep generated SVG factories in this registry instead of exposing artwork
 * to controls and product code. Add entries here only as the application uses
 * them so the renderer bundle remains tree-shakable.
 */
export const lxiconsLibrary = {
  add: register("add", svg.add),
  agent: register("agent", svg.agent),
  arrowUp: register("arrow-up", svg.arrowUp),
  browserWeb: register("browser-web", svg.browserWeb),
  chat: register("chat", svg.chat),
  chatFilled: register("chat-filled", svg.chatFilled),
  check: register("check", svg.check),
  chevronDown: register("chevron-down", svg.chevronDown),
  chevronRight: register("chevron-right", svg.chevronRight),
  close: register("close", svg.close),
  dropdownIndicator: register("dropdown-indicator", svg.chevronDown),
  ellipsis: register("ellipsis", svg.ellipsis),
  files: register("files", svg.files),
  gear: register("gear", svg.gear),
  gitBranch: register("git-branch", svg.gitBranch),
  gitCommit: register("git-commit", svg.gitCommit),
  history: register("history", svg.history),
  layoutSidebarLeft: register("layout-sidebar-left", svg.layoutSidebarLeft),
  layoutSidebarLeftOff: register("layout-sidebar-left-off", svg.layoutSidebarLeftOff),
  layoutSidebarRight: register("layout-sidebar-right", svg.layoutSidebarRight),
  layoutSidebarRightOff: register("layout-sidebar-right-off", svg.layoutSidebarRightOff),
  layoutPanel: register("layout-panel", svg.layoutPanel),
  layoutPanelOff: register("layout-panel-off", svg.layoutPanelOff),
  linkExternal: register("link-external", svg.linkExternal),
  menu: register("menu", svg.menu),
  model: register("model", svg.model),
  refresh: register("refresh", svg.refresh),
  repoFetch: register("repo-fetch", svg.repoFetch),
  repoPull: register("repo-pull", svg.repoPull),
  repoPush: register("repo-push", svg.repoPush),
  search: register("search", svg.search),
  screenFull: register("screen-full", svg.screenFull),
  screenNormal: register("screen-normal", svg.screenNormal),
  settings: register("settings", svg.settings),
  splitHorizontal: register("split-horizontal", svg.splitHorizontal),
  start: register("start", svg.start),
  submenuIndicator: register("submenu-indicator", svg.chevronRight),
  terminal: register("terminal", svg.terminal),
  terminalCmd: register("terminal-cmd", svg.terminalCmd),
  terminalGitBash: register("terminal-git-bash", svg.terminalGitBash),
  trash: register("trash", svg.trash),
} as const;
