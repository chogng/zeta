import { lxiconsLibrary } from "../../../../base/common/lxiconsLibrary.js";
import { Action2, MenuId, registerAction2 } from "../../../../platform/actions/common/actions.js";
import { Keybinding, logicalKey } from "../../../../base/common/keybindings.js";
import type { ServicesAccessor } from "../../../../platform/instantiation/common/instantiation.js";
import { IEditorPart } from "./editorPart.js";
import { IUntitledTextEditorService } from "../../../services/untitled/common/untitledTextEditorService.js";

export const SplitEditorHorizontalCommandId =
  "workbench.action.splitEditorHorizontal";

registerAction2(class SplitEditorHorizontalAction extends Action2 {
  constructor() {
    super({
      id: SplitEditorHorizontalCommandId,
      title: "Split Editor Horizontal",
      tooltip: "Split Editor Horizontal",
      icon: lxiconsLibrary.splitHorizontal,
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

export const NewUntitledTextEditorCommandId =
  "workbench.action.files.newUntitledFile";

registerAction2(class NewUntitledTextEditorAction extends Action2 {
  constructor() {
    super({
      id: NewUntitledTextEditorCommandId,
      title: "New Untitled Text Editor",
      tooltip: "New Untitled Text Editor",
      icon: lxiconsLibrary.add,
      f1: true,
      keybinding: {
        primary: Keybinding.single(logicalKey("n", { primaryKey: true })),
      },
    });
  }

  override run(accessor: ServicesAccessor): Promise<void> {
    const untitled = accessor.get(IUntitledTextEditorService).create();
    return accessor.get(IEditorPart).openEditor({
      resource: untitled.resource,
      label: untitled.label,
      initialText: untitled.initialText,
      languageId: untitled.languageId,
    }).then(() => undefined);
  }
});
