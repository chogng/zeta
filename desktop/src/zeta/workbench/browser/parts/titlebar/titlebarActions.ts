import { lxiconsLibrary } from "../../../../base/common/lxiconsLibrary.js";
import { Action2, MenuId, registerAction2 } from "../../../../platform/actions/common/actions.js";
import type { ServicesAccessor } from "../../../../platform/instantiation/common/instantiation.js";
import { AuxiliaryBarVisibleContext, PanelVisibleContext, SideBarVisibleContext } from "../../../common/contextkeys.js";
import { IWorkbenchLayoutService } from "../../layout.js";

export const ToggleSideBarCommandId = "workbench.action.toggleSideBar";
export const ToggleAuxiliaryBarCommandId = "workbench.action.toggleAuxiliaryBar";
export const TogglePanelCommandId = "workbench.action.togglePanel";

registerAction2(class ToggleSideBarAction extends Action2 {
  constructor() {
    super({
      id: ToggleSideBarCommandId,
      title: "Show Primary Side Bar",
      tooltip: "Show Primary Side Bar",
      icon: lxiconsLibrary.layoutSidebarLeftOff,
      toggled: {
        condition: SideBarVisibleContext.isEqualTo(true),
        title: "Hide Primary Side Bar",
        tooltip: "Hide Primary Side Bar",
        icon: lxiconsLibrary.layoutSidebarLeft,
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
      icon: lxiconsLibrary.layoutSidebarRightOff,
      toggled: {
        condition: AuxiliaryBarVisibleContext.isEqualTo(true),
        title: "Hide Secondary Side Bar",
        tooltip: "Hide Secondary Side Bar",
        icon: lxiconsLibrary.layoutSidebarRight,
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

registerAction2(class TogglePanelAction extends Action2 {
  constructor() {
    super({
      id: TogglePanelCommandId,
      title: "Show Panel",
      tooltip: "Show Panel",
      icon: lxiconsLibrary.layoutPanelOff,
      toggled: {
        condition: PanelVisibleContext.isEqualTo(true),
        title: "Hide Panel",
        tooltip: "Hide Panel",
        icon: lxiconsLibrary.layoutPanel,
      },
      menu: [
        {
          id: MenuId.TitleBar,
          group: "navigation",
          order: 9,
        },
        {
          id: MenuId.MenubarViewMenu,
          group: "2_appearance",
          order: 11,
        },
      ],
      f1: true,
    });
  }

  override run(accessor: ServicesAccessor): void {
    const layout = accessor.get(IWorkbenchLayoutService);
    if (layout.isPartVisible("panel")) {
      layout.hidePart("panel");
    } else {
      layout.showPart("panel");
    }
  }
});
