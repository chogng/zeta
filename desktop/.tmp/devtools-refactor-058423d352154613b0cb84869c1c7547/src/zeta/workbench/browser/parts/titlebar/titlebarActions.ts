import { LxIcon } from "../../../../base/common/lxicons.js";
import {
  Action2,
  MenuId,
  registerAction2,
} from "../../../../platform/actions/common/actions.js";
import type {
  ServicesAccessor,
} from "../../../../platform/instantiation/common/instantiation.js";
import {
  AuxiliaryBarVisibleContext,
  SideBarVisibleContext,
} from "../../../common/contextkeys.js";
import { IWorkbenchLayoutService } from "../../layout.js";

export const ToggleSideBarCommandId = "workbench.action.toggleSideBar";
export const ToggleAuxiliaryBarCommandId =
  "workbench.action.toggleAuxiliaryBar";

registerAction2(class ToggleSideBarAction extends Action2 {
  constructor() {
    super({
      id: ToggleSideBarCommandId,
      title: "Show Primary Side Bar",
      tooltip: "Show Primary Side Bar",
      icon: LxIcon.layoutSidebarLeftOff,
      toggled: {
        condition: SideBarVisibleContext.isEqualTo(true),
        title: "Hide Primary Side Bar",
        tooltip: "Hide Primary Side Bar",
        icon: LxIcon.layoutSidebarLeft,
      },
      menu: [
        {
          id: MenuId.TitleBarLeft,
          group: "navigation",
          order: 10,
        },
        {
          id: MenuId.MenubarViewMenu,
          group: "2_appearance",
          order: 9,
        },
      ],
      f1: true,
    });
  }

  override run(accessor: ServicesAccessor): void {
    const layout = accessor.get(IWorkbenchLayoutService);
    if (layout.isPartVisible("sidebar")) {
      layout.hidePart("sidebar");
    } else {
      layout.showPart("sidebar");
    }
  }
});

registerAction2(class ToggleAuxiliaryBarAction extends Action2 {
  constructor() {
    super({
      id: ToggleAuxiliaryBarCommandId,
      title: "Show Secondary Side Bar",
      tooltip: "Show Secondary Side Bar",
      icon: LxIcon.chat,
      toggled: {
        condition: AuxiliaryBarVisibleContext.isEqualTo(true),
        title: "Hide Secondary Side Bar",
        tooltip: "Hide Secondary Side Bar",
        icon: LxIcon.chatFilled,
      },
      menu: [
        {
          id: MenuId.TitleBar,
          group: "navigation",
          order: 10,
        },
        {
          id: MenuId.MenubarViewMenu,
          group: "2_appearance",
          order: 10,
        },
      ],
      f1: true,
    });
  }

  override run(accessor: ServicesAccessor): void {
    const layout = accessor.get(IWorkbenchLayoutService);
    if (layout.isPartVisible("auxiliarybar")) {
      layout.hidePart("auxiliarybar");
    } else {
      layout.showPart("auxiliarybar");
    }
  }
});
