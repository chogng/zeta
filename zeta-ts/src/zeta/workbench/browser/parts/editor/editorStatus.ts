import { DisposableOwner, DisposableSlot, type IDisposable } from "../../../../base/common/lifecycle.js";
import { isEditorPaneWithStatus } from "./editorPane.js";
import type { IEditorPart } from "./editorPart.js";
import { StatusbarAlignment, type IStatusbarEntry, type IStatusbarEntryAccessor, type IStatusbarService } from "../../../services/statusbar/browser/statusbar.js";

/** Projects the active editor's cursor, format, language, and save state. */
export class EditorStatusContribution extends DisposableOwner {
	private readonly cursor = this.own(new DisposableSlot<IStatusbarEntryAccessor>());
	private readonly format = this.own(new DisposableSlot<IStatusbarEntryAccessor>());
	private readonly language = this.own(new DisposableSlot<IStatusbarEntryAccessor>());
	private readonly state = this.own(new DisposableSlot<IStatusbarEntryAccessor>());
	private readonly paneListener = this.own(new DisposableSlot<IDisposable>());

	constructor(private readonly editorPart: IEditorPart, private readonly statusbar: IStatusbarService) {
		super();
		this.own(editorPart.onDidChangeEditors(() => this.update()));
		this.update();
	}

	private update(): void {
		const input = this.editorPart.activeInput;
		const pane = this.editorPart.activePane;
		this.paneListener.replace(isEditorPaneWithStatus(pane) ? pane.onDidChangeStatus(() => this.updateEntries()) : undefined);
		if (!input || !pane) {
			this.cursor.clear();
			this.format.clear();
			this.language.clear();
			this.state.clear();
			return;
		}
		this.updateEntries();
	}

	private updateEntries(): void {
		const input = this.editorPart.activeInput;
		const pane = this.editorPart.activePane;
		if (!input || !pane) return;
		const status = isEditorPaneWithStatus(pane) ? pane.getStatus() : undefined;
		const cursorText = status?.lineNumber !== undefined && status.columnNumber !== undefined
			? `Ln ${status.lineNumber}, Col ${status.columnNumber}${status.selectionCount ? ` (${status.selectionCount} selections)` : ""}`
			: undefined;
		this.setEntry(this.cursor, cursorText ? { text: cursorText, ariaLabel: cursorText } : undefined, "zeta.status.editor.cursor", 80);
		const formatText = [status?.endOfLine, status?.encoding].filter(Boolean).join("  ");
		this.setEntry(this.format, formatText ? { text: formatText, ariaLabel: `Editor format ${formatText}` } : undefined, "zeta.status.editor.format", 70);
		const language = status?.languageId ?? input.languageId;
		this.setEntry(this.language, language ? { text: languageDisplayName(language), ariaLabel: `Language ${languageDisplayName(language)}` } : undefined, "zeta.status.editor.language", 60);
		const workingCopy = pane.workingCopy;
		const stateText = workingCopy?.hasExternalChange ? "Conflict" : workingCopy?.isDirty ? "Unsaved" : input.readOnly ? "Read-only" : undefined;
		this.setEntry(this.state, stateText ? { text: stateText, ariaLabel: `Editor state ${stateText}`, tooltip: stateText === "Conflict" ? "The file changed on disk while this editor has unsaved changes." : undefined } : undefined, "zeta.status.editor.state", 90);
	}

	private setEntry(slot: DisposableSlot<IStatusbarEntryAccessor>, entry: IStatusbarEntry | undefined, id: string, priority: number): void {
		if (!entry) {
			slot.clear();
			return;
		}
		if (slot.value) slot.value.update(entry);
		else slot.replace(this.statusbar.addEntry(entry, { id, alignment: StatusbarAlignment.Right, priority, compactGroup: "editor" }));
	}
}

function languageDisplayName(languageId: string): string {
	const known: Readonly<Record<string, string>> = {
		plaintext: "Plain Text",
		typescript: "TypeScript",
		javascript: "JavaScript",
		json: "JSON",
		jsonc: "JSON with Comments",
		markdown: "Markdown",
	};
	return known[languageId] ?? languageId;
}
