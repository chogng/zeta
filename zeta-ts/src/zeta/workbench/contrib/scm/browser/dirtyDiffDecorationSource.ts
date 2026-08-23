import { type Event } from "../../../../base/common/event.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type URI } from "../../../../base/common/uri.js";
import { createAsterDecorationSource, DecorationPresentation, type DecorationSource, type OwnedDecorationSource, type ResolvedDecoration } from "../../../../editor/browser/viewparts/decorations/decorationPresentation.js";
import { RustDiffComputationService } from "../../../../editor/browser/services/rustDiffComputationService.js";
import { TextPosition, TextRange } from "../../../../editor/common/core/text.js";
import { LineDiffKind, type LineDiff, type LineDiffRow } from "../../../../editor/common/diff/lineDiff.js";
import { TextDecorationCollection } from "../../../../editor/common/model/decorationCollection.js";
import { type TextModel } from "../../../../editor/common/model/textModel.js";
import { TrackedRangeStickiness } from "../../../../editor/common/model/trackedRange.js";
import { type IDiffApi } from "../../../../platform/diff/common/diffApi.js";
import { getRemoteWorkspacePath, isRemoteResource } from "../../../../platform/remote/common/remote.js";
import { type IGitService } from "../../../services/git/common/gitService.js";

const MODEL_REFRESH_DELAY_MS = 150;

/** VS Code-style Quick Diff decorations projected from Git's index to the live editor model. */
export class DirtyDiffDecorationSource extends DisposableOwner implements OwnedDecorationSource {
	private readonly collection: TextDecorationCollection<DecorationPresentation>;
	private readonly source: DecorationSource;
	private readonly diffService: RustDiffComputationService;
	private refreshGeneration = 0;
	private activeRequest: AbortController | undefined;
	private refreshHandle: ReturnType<typeof globalThis.setTimeout> | undefined;
	private disposed = false;

	readonly onDidChange: Event<void>;

	constructor(private readonly resource: URI, private readonly model: TextModel, private readonly gitService: IGitService, diffApi: IDiffApi) {
		super();
		this.collection = this.own(new TextDecorationCollection<DecorationPresentation>(model));
		this.source = createAsterDecorationSource(this.collection, decoration => decoration.metadata, decoration => hoverText(decoration.metadata));
		this.diffService = this.own(new RustDiffComputationService(diffApi));
		this.onDidChange = this.source.onDidChange;
		this.own(model.onDidChange(() => this.scheduleRefresh(MODEL_REFRESH_DELAY_MS)));
		this.own(gitService.onDidChangeStatus(() => this.scheduleRefresh(0)));
		this.own(gitService.onDidBecomeReady(() => this.scheduleRefresh(0)));
		this.defer(() => {
			this.disposed = true;
			this.refreshGeneration += 1;
			this.activeRequest?.abort("dirtyDiffDisposed");
			this.activeRequest = undefined;
			if (this.refreshHandle !== undefined) globalThis.clearTimeout(this.refreshHandle);
			this.refreshHandle = undefined;
		});
		this.scheduleRefresh(0);
	}

	get decorations(): readonly ResolvedDecoration[] {
		return this.source.decorations;
	}

	/** Recomputes against the latest Git index and live model revision. */
	async refresh(): Promise<void> {
		if (this.refreshHandle !== undefined) globalThis.clearTimeout(this.refreshHandle);
		this.refreshHandle = undefined;
		await this.runRefresh();
	}

	private scheduleRefresh(delayMs: number): void {
		if (this.disposed) return;
		if (this.refreshHandle !== undefined) globalThis.clearTimeout(this.refreshHandle);
		this.refreshHandle = globalThis.setTimeout(() => {
			this.refreshHandle = undefined;
			void this.runRefresh();
		}, delayMs);
	}

	private async runRefresh(): Promise<void> {
		if (this.disposed) return;
		const generation = ++this.refreshGeneration;
		this.activeRequest?.abort("dirtyDiffSuperseded");
		const controller = new AbortController();
		this.activeRequest = controller;
		const modelVersion = this.model.version;
		try {
			const status = await this.gitService.status();
			const path = workspaceRelativePath(this.resource, status.workspacePath);
			if (!path) {
				this.accept(generation, modelVersion, Object.freeze([]));
				return;
			}
			const change = status.changes.find(candidate => normalizePath(candidate.path) === path);
			if (!change || change.conflicted) {
				this.accept(generation, modelVersion, Object.freeze([]));
				return;
			}
			const comparison = change.worktreeStatus !== "unmodified" ? "unstaged" : "staged";
			const file = await this.gitService.changeFile(path, comparison);
			if (file.original.kind === "binary" || file.modified.kind === "binary") {
				this.accept(generation, modelVersion, Object.freeze([]));
				return;
			}
			const original = file.original.kind === "text" ? file.original.text : "";
			const diff = await this.diffService.compute({
				original: { version: status.revision, text: original },
				modified: { version: modelVersion, text: this.model.getText() },
			}, controller.signal);
			this.accept(generation, modelVersion, decorationsForDiff(this.model, diff));
		} catch (error) {
			if (controller.signal.aborted || this.disposed) return;
			// Git can be unavailable during workspace startup; retain the last valid projection.
			void error;
		} finally {
			if (this.activeRequest === controller) this.activeRequest = undefined;
		}
	}

	private accept(generation: number, modelVersion: number, specs: ReturnType<typeof decorationsForDiff>): void {
		if (this.disposed || generation !== this.refreshGeneration || modelVersion !== this.model.version) return;
		this.collection.replaceAll(specs);
	}
}

function decorationsForDiff(model: TextModel, diff: LineDiff) {
	const byLine = new Map<number, DecorationPresentation>();
	for (let rowIndex = 0; rowIndex < diff.rows.length; rowIndex += 1) {
		const row = diff.rows[rowIndex]!;
		if (row.kind === LineDiffKind.Unchanged) continue;
		const lineIndex = row.modifiedLineIndex ?? deletionAnchor(diff.rows, rowIndex, model.lineCount);
		const presentation = presentationForRow(row);
		const current = byLine.get(lineIndex);
		if (!current || diffPresentationPriority(presentation) > diffPresentationPriority(current)) byLine.set(lineIndex, presentation);
	}
	return Object.freeze([...byLine.entries()].sort(([left], [right]) => left - right).map(([lineIndex, presentation]) => Object.freeze({
		range: TextRange.from(TextPosition.at(lineIndex, 0), TextPosition.at(lineIndex, model.getLineLength(lineIndex))),
		stickiness: TrackedRangeStickiness.NeverGrowsAtEdges,
		metadata: presentation,
	})));
}

function deletionAnchor(rows: readonly LineDiffRow[], rowIndex: number, lineCount: number): number {
	for (let index = rowIndex + 1; index < rows.length; index += 1) {
		const lineIndex = rows[index]?.modifiedLineIndex;
		if (lineIndex !== undefined) return lineIndex;
	}
	for (let index = rowIndex - 1; index >= 0; index -= 1) {
		const lineIndex = rows[index]?.modifiedLineIndex;
		if (lineIndex !== undefined) return lineIndex;
	}
	return Math.max(0, lineCount - 1);
}

function presentationForRow(row: LineDiffRow): DecorationPresentation.DiffAdded | DecorationPresentation.DiffModified | DecorationPresentation.DiffDeleted {
	switch (row.kind) {
		case LineDiffKind.Added: return DecorationPresentation.DiffAdded;
		case LineDiffKind.Modified: return DecorationPresentation.DiffModified;
		case LineDiffKind.Removed: return DecorationPresentation.DiffDeleted;
		case LineDiffKind.Unchanged: throw new TypeError("Unchanged rows do not create Quick Diff decorations");
	}
}

function diffPresentationPriority(presentation: DecorationPresentation): number {
	switch (presentation) {
		case DecorationPresentation.DiffDeleted: return 3;
		case DecorationPresentation.DiffModified: return 2;
		case DecorationPresentation.DiffAdded: return 1;
		default: return 0;
	}
}

function hoverText(presentation: DecorationPresentation): string | undefined {
	switch (presentation) {
		case DecorationPresentation.DiffAdded: return "Added line";
		case DecorationPresentation.DiffModified: return "Modified line";
		case DecorationPresentation.DiffDeleted: return "Deleted line";
		default: return undefined;
	}
}

function workspaceRelativePath(resource: URI, workspacePath: string): string | undefined {
	if (resource.scheme !== "file" && !isRemoteResource(resource)) return undefined;
	const resourcePath = normalizePath(isRemoteResource(resource) ? getRemoteWorkspacePath(resource) : resource.fsPath);
	const workspace = normalizePath(workspacePath).replace(/\/$/u, "");
	const compareResource = /^[A-Za-z]:\//u.test(resourcePath) ? resourcePath.toLowerCase() : resourcePath;
	const compareWorkspace = /^[A-Za-z]:\//u.test(workspace) ? workspace.toLowerCase() : workspace;
	const prefix = `${compareWorkspace}/`;
	if (!compareResource.startsWith(prefix)) return undefined;
	return resourcePath.slice(workspace.length + 1);
}

function normalizePath(value: string): string {
	return value.replaceAll("\\", "/").replace(/\/{2,}/gu, "/");
}
