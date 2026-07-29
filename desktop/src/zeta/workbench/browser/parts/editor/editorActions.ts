import { LxIcon } from "../../../../base/common/lxicons.js";
import { Action2, MenuId, registerAction2 } from "../../../../platform/actions/common/actions.js";
import type { ServicesAccessor } from "../../../../platform/instantiation/common/instantiation.js";
import { IEditorPart } from "./editorPart.js";

export const SplitEditorHorizontalCommandId =
  "workbench.action.splitEditorHorizontal";

registerAction2(class SplitEditorHorizontalAction extends Action2 {
  constructor() {
    super({
      id: SplitEditorHorizontalCommandId,
      title: "Split Editor Horizontal",
      tooltip: "Split Editor Horizontal",
      icon: LxIcon.splitHorizontal,
      f1: true,
      menu: {
        id: MenuId.EditorTitle,
        group: "navigation",
        order: 1,
      },
    });
  }

  override run(accessor: ServicesAccessor): Promise<void> {
    return accessor.get(IEditorPart).splitActiveGroupHorizontal();
  }
});
