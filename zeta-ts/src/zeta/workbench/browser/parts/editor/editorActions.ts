import { lxiconsLibrary } from "../../../../base/common/lxiconsLibrary.js";
import { DisposableStore } from "../../../../base/common/lifecycle.js";
import { Action2, MenuId, registerAction2 } from "../../../../platform/actions/common/actions.js";
import { Keybinding, logicalKey } from "../../../../base/common/keybindings.js";
import type { ServicesAccessor } from "../../../../platform/instantiation/common/instantiation.js";
import { IQuickInputService, type IQuickPickItem } from "../../../../platform/quickinput/common/quickInput.js";
import { IEditorPart } from "./editorPart.js";
import type { ExtensionFileTemplateDefinition } from "../../../services/extensions/common/extensionFileTemplate.js";
import { IExtensionService } from "../../../services/extensions/common/extensionService.js";
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

export const NewFileFromTemplateCommandId = "workbench.action.files.newFileFromTemplate";

interface FileTemplateQuickPickItem extends IQuickPickItem {
	readonly template: ExtensionFileTemplateDefinition;
}

registerAction2(class NewFileFromTemplateAction extends Action2 {
	constructor() {
		super({
			id: NewFileFromTemplateCommandId,
			title: "New File from Template",
			tooltip: "New File from Template",
			f1: true,
			menu: { id: MenuId.MenubarFileMenu, group: "1_file", order: 0 },
		});
	}

	override run(accessor: ServicesAccessor): void {
		const templates = accessor.get(IExtensionService).fileTemplates.currentCatalog.templates;
		if (templates.length === 0) return;
		const picker = accessor.get(IQuickInputService).createQuickPick<FileTemplateQuickPickItem>();
		const disposables = new DisposableStore();
		disposables.add(picker);
		picker.placeholder = "Select a file template";
		picker.items = templates.map(template => ({
			template,
			label: template.label,
			description: template.extensionId,
			...(template.description === undefined ? {} : { detail: template.description }),
		}));
		disposables.add(picker.onDidAccept(item => {
			picker.hide();
			const untitled = accessor.get(IUntitledTextEditorService).create({ initialText: item.template.body, languageId: item.template.languageId });
			void accessor.get(IEditorPart).openEditor({
				resource: untitled.resource,
				label: untitled.label,
				initialText: untitled.initialText,
				languageId: untitled.languageId,
			}).catch(error => console.error("Could not create file from extension template", error));
		}));
		disposables.add(picker.onDidHide(() => disposables.dispose()));
		picker.show();
	}
});

/** Saves the active pane through the Workbench-owned editor lifecycle. */
export const SaveActiveEditorCommandId =
	"workbench.action.files.save";

registerAction2(class SaveActiveEditorAction extends Action2 {
	constructor() {
		super({
			id: SaveActiveEditorCommandId,
			title: "Save",
			tooltip: "Save",
			f1: true,
			menu: {
				id: MenuId.MenubarFileMenu,
				group: "1_file",
				order: 1,
			},
			keybinding: {
				primary: Keybinding.single(logicalKey("s", { primaryKey: true })),
			},
		});
	}

	override run(accessor: ServicesAccessor): Promise<void> {
		return accessor.get(IEditorPart).saveActiveEditor();
	}
});
