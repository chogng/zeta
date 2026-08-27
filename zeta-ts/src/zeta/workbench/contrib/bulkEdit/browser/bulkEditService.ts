import { Disposable, toDisposable } from "../../../../base/common/lifecycle.js";
import { normalizeLanguageWorkspaceEdit, type LanguageWorkspaceEdit } from "../../../../editor/common/languages/languageWorkspaceEdit.js";
import { type IWorkspaceEditService, type WorkspaceEditResult } from "../../../services/language/common/workspaceEditService.js";
import { type BulkEditApplyOptions, type BulkEditPreviewHandler, type BulkEditResult, IBulkEditService } from "../common/bulkEdit.js";

/** Adds Workbench preview policy to the ordered workspace-edit transaction. */
export class BrowserBulkEditService extends Disposable implements IBulkEditService {
	private previewHandler: BulkEditPreviewHandler | undefined;

	constructor(private readonly workspaceEdits: IWorkspaceEditService) {
		super();
		if (!workspaceEdits || typeof workspaceEdits.apply !== "function") throw new TypeError("Bulk edit service requires a workspace edit applier");
		this._register(toDisposable(() => { this.previewHandler = undefined; }));
	}

	hasPreviewHandler(): boolean {
		return this.previewHandler !== undefined;
	}

	setPreviewHandler(handler: BulkEditPreviewHandler): ReturnType<typeof toDisposable> {
		if (typeof handler !== "function") throw new TypeError("Bulk edit preview handler must be a function");
		const previous = this.previewHandler;
		this.previewHandler = handler;
		return toDisposable(() => {
			if (this.previewHandler === handler) this.previewHandler = previous;
		});
	}

	async apply(value: LanguageWorkspaceEdit, options: BulkEditApplyOptions = {}): Promise<BulkEditResult> {
		let edit = normalizeLanguageWorkspaceEdit(value);
		const signal = options.signal ?? new AbortController().signal;
		const preview = options.preview ?? (edit.entries.length > 1 && this.previewHandler ? "always" : "never");
		if (preview === "always") {
			const previewHandler = this.previewHandler;
			if (!previewHandler) throw new Error("Bulk edit preview is not available");
			const accepted = await previewHandler(edit, signal);
			if (!accepted) return { resources: Object.freeze([]), applied: false };
			edit = normalizeLanguageWorkspaceEdit(accepted);
		}
		const result: WorkspaceEditResult = await this.workspaceEdits.apply(edit, signal);
		return { ...result, applied: edit.entries.length > 0 };
	}
}
