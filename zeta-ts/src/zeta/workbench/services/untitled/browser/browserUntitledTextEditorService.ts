import { Emitter } from "../../../../base/common/event.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { URI } from "../../../../base/common/uri.js";
import { type IUntitledTextEditor, type IUntitledTextEditorService, type UntitledTextEditorOptions, UNTITLED_TEXT_EDITOR_SCHEME } from "../common/untitledTextEditorService.js";

/** Browser-side owner for Workbench untitled editor identities. */
export class BrowserUntitledTextEditorService extends DisposableOwner implements IUntitledTextEditorService {
	private readonly editors = new Map<string, IUntitledTextEditor>();
	private readonly _onDidCreate = this.own(new Emitter<IUntitledTextEditor>());
	private nextUntitledNumber = 1;

	readonly onDidCreate = this._onDidCreate.event;

	create(options: UntitledTextEditorOptions = {}): IUntitledTextEditor {
		validateOptions(options);
		const number = this.nextUntitledNumber++;
		const label = `Untitled-${number}`;
		const editor = Object.freeze({
			resource: URI.parse(`${UNTITLED_TEXT_EDITOR_SCHEME}:/${label}`),
			label,
			initialText: options.initialText ?? "",
			languageId: options.languageId,
		});
		this.editors.set(editor.resource.toString(), editor);
		this._onDidCreate.fire(editor);
		return editor;
	}

	get(resource: URI): IUntitledTextEditor | undefined {
		if (!this.isUntitled(resource)) return undefined;
		return this.editors.get(resource.toString());
	}

	isUntitled(resource: URI): boolean {
		return resource.scheme === UNTITLED_TEXT_EDITOR_SCHEME;
	}
}

function validateOptions(options: UntitledTextEditorOptions): void {
	if (!options || typeof options !== "object") {
		throw new TypeError("Untitled editor options must be an object");
	}
	if (options.initialText !== undefined && typeof options.initialText !== "string") {
		throw new TypeError("Untitled editor initial text must be a string");
	}
	if (options.languageId !== undefined && (typeof options.languageId !== "string" || options.languageId.trim().length === 0)) {
		throw new TypeError("Untitled editor language id must be a non-empty string");
	}
}
