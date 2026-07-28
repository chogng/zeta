import {
  MenuId,
  MenusRegistry,
} from "../../../../platform/actions/common/actions.js";

MenusRegistry.appendMenuItem(MenuId.MenubarMainMenu, {
  title: "File",
  submenu: MenuId.MenubarFileMenu,
  group: "1_file",
  order: 1,
});
