import {
	MenuId,
	MenusRegistry,
} from "../../../../platform/actions/common/actions.js";
import { localizedString } from "../../../../platform/action/common/action.js";

const applicationMenus = [
	[localizedString("zeta.menu", "file", "File"), MenuId.MenubarFileMenu],
	[localizedString("zeta.menu", "edit", "Edit"), MenuId.MenubarEditMenu],
	[localizedString("zeta.menu", "selection", "Selection"), MenuId.MenubarSelectionMenu],
	[localizedString("zeta.menu", "view", "View"), MenuId.MenubarViewMenu],
	[localizedString("zeta.menu", "go", "Go"), MenuId.MenubarGoMenu],
	[localizedString("zeta.menu", "run", "Run"), MenuId.MenubarRunMenu],
	[localizedString("zeta.menu", "terminal", "Terminal"), MenuId.MenubarTerminalMenu],
	[localizedString("zeta.menu", "help", "Help"), MenuId.MenubarHelpMenu],
] as const;

for (const [index, [title, submenu]] of applicationMenus.entries()) {
	MenusRegistry.appendMenuItem(MenuId.MenubarMainMenu, {
		title,
		submenu,
		group: "navigation",
		order: index + 1,
	});
}
