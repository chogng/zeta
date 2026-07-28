import {
  MenuId,
  MenusRegistry,
} from "../../../../platform/actions/common/actions.js";

const applicationMenus = [
  ["File", MenuId.MenubarFileMenu],
  ["Edit", MenuId.MenubarEditMenu],
  ["Selection", MenuId.MenubarSelectionMenu],
  ["View", MenuId.MenubarViewMenu],
  ["Go", MenuId.MenubarGoMenu],
  ["Run", MenuId.MenubarRunMenu],
  ["Terminal", MenuId.MenubarTerminalMenu],
  ["Help", MenuId.MenubarHelpMenu],
] as const;

for (const [index, [title, submenu]] of applicationMenus.entries()) {
  MenusRegistry.appendMenuItem(MenuId.MenubarMainMenu, {
    title,
    submenu,
    group: "navigation",
    order: index + 1,
  });
}
