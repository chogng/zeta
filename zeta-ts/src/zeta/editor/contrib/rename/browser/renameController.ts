import "./media/rename.css";
import { registerEditorContribution } from "../../../browser/editorContribution.js";
import { addDisposableListener, stopEvent, h } from "../../../../base/browser/dom.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type URI } from "../../../../base/common/uri.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { createEditorEditCommand } from "../../../common/commands/editorCommand.js";
import { type EditorViewport } from "../../../browser/view/editorViewport.js";
import { type RenameService } from "../common/rename.js";
import { type LanguageWorkspaceEdit } from "../../../common/languages/languageWorkspaceEdit.js";

/** Owns the local rename input and applies provider edits through the cursor command contract. */
export class RenameController extends DisposableOwner {
	private readonly element: HTMLDivElement;
	private readonly input: HTMLInputElement;
	private readonly status: HTMLSpanElement;
	private request: AbortController | undefined;

	constructor(private readonly editorInput: HTMLTextAreaElement, private readonly viewport: EditorViewport, private readonly selections: EditorSelectionController, private readonly service: RenameService, private readonly languageId: string, private readonly resource: URI, private readonly applyWorkspaceEdit: ((edit: LanguageWorkspaceEdit) => void | Promise<void>) | undefined, private readonly onError: (error: unknown) => void = error => console.error("Editor rename failed", error)) {
		super();
		if (viewport.textModel !== selections.textModel) throw new TypeError("Aster rename dependencies must share one text model");
		const ownerDocument = viewport.element.ownerDocument;
		this.element = h(ownerDocument, "div");
		this.element.className = "aster-editor-rename";
		this.element.hidden = true;
		this.input = h(ownerDocument, "input");
		this.input.className = "aster-editor-rename-input";
		this.input.type = "text";
		this.input.setAttribute("aria-label", "New symbol name");
		this.status = h(ownerDocument, "span");
		this.status.className = "aster-editor-rename-status";
		this.status.setAttribute("aria-live", "polite");
		this.element.append(this.input, this.status);
		viewport.element.append(this.element);
		this.defer(() => this.element.remove());
		this.own(addDisposableListener(editorInput, "keydown", event => {
			if (event.defaultPrevented || event.isComposing || event.altKey || event.ctrlKey || event.metaKey || event.key !== "F2") return;
			stopEvent(event);
			void this.open();
		}));
		this.own(addDisposableListener(this.element, "keydown", event => this.handleWidgetKeydown(event)));
	}

	private async open(): Promise<void> {
		this.cancelRequest();
		const request = this.request = new AbortController();
		try {
			const active = this.selections.selections.primary.active;
			const preparation = await this.service.prepareRename(this.languageId, active, request.signal);
			if (request.signal.aborted) return;
			if (!preparation) {
				this.viewport.announceAccessibilityStatus("Rename is not available at this position.");
				return;
			}
			this.element.hidden = false;
			this.status.textContent = preparation.placeholder;
			this.input.value = preparation.placeholder;
			const coordinates = this.viewport.getPositionContentCoordinates(preparation.range.start);
			this.element.style.left = `${Math.max(8, coordinates.left - this.viewport.viewportLayout.scrollPosition.left)}px`;
			this.element.style.top = `${Math.max(8, coordinates.top - this.viewport.viewportLayout.scrollPosition.top + coordinates.height + 4)}px`;
			this.input.focus({ preventScroll: true });
			this.input.select();
		} catch (error) {
			if (!request.signal.aborted) this.onError(error);
		}
	}

	private handleWidgetKeydown(event: KeyboardEvent): void {
		if (event.defaultPrevented || event.isComposing) return;
		if (event.key === "Escape") {
			stopEvent(event);
			this.close();
			return;
		}
		if (event.key !== "Enter" || event.ctrlKey || event.metaKey || event.altKey) return;
		stopEvent(event);
		void this.commit();
	}

	private async commit(): Promise<void> {
		const newName = this.input.value.trim();
		if (newName.length === 0) {
			this.status.textContent = "Name cannot be empty";
			return;
		}
		const request = this.request = new AbortController();
		try {
			const active = this.selections.selections.primary.active;
			const edit = await this.service.provideRenameEdits(this.languageId, active, newName, request.signal);
			if (request.signal.aborted) return;
			if (this.applyWorkspaceEdit) {
				await this.applyWorkspaceEdit(edit);
			} else {
				const documentEdit = edit.entries.find(candidate => candidate.kind === "textDocument" && candidate.resource.toString() === this.resource.toString());
				if (edit.entries.length !== 1 || !documentEdit || documentEdit.kind !== "textDocument") throw new Error("This editor host cannot apply a multi-resource rename");
				const command = createEditorEditCommand(this.viewport.textModel, this.selections.selections, documentEdit.edits);
				if (command) this.selections.execute(command);
			}
			this.close();
		} catch (error) {
			if (!request.signal.aborted) this.onError(error);
		}
	}

	private close(): void {
		this.cancelRequest();
		this.element.hidden = true;
		this.editorInput.focus({ preventScroll: true });
	}

	private cancelRequest(): void {
		this.request?.abort();
		this.request = undefined;
	}
}

registerEditorContribution({ id: "editor.contrib.rename", install: context => {
	if (context.kind !== "text") return;
	const service = context.own(context.languageFeaturesService.createRenameService(context.model, context.options.input.resource));
	context.own(new RenameController(context.textInput.element, context.viewport, context.selections, service, context.languageId, context.options.input.resource, context.options.onApplyWorkspaceEdit, context.onLanguageError));
} });
