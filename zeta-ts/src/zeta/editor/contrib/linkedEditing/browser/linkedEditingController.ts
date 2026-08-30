import { Position } from "../../../common/core/position.js";
import "./linkedEditing.css";
import { registerTextEditorCapabilityContribution } from "../../../browser/editorExtensions.js";
import { type EditorView } from '../../../browser/editorView.js';
import { type View } from "../../../browser/view.js";
import { addDisposableListener, stopEvent } from "../../../../base/browser/dom.js";
import { Disposable, DisposableStore, toDisposable } from "../../../../base/common/lifecycle.js";
import { extendEditorEditCommand } from "../../../common/commands/editorCommand.js";
import { type EditorEditCommand } from "../../../common/commands/editorEditCommand.js";
import { type CursorsController } from "../../../common/cursor/cursor.js";
import { Range } from "../../../common/core/range.js";

import { type TrackedRange } from "../../../common/model/trackedRange.js";
import { LinkedEditingService } from "../common/languageLinkedEditing.js";
import { TrackedRangeStickiness } from '../../../common/model.js';

/** Synchronizes provider-declared linked ranges through one atomic native-input transaction. */
export class LinkedEditingController extends Disposable {
	private readonly ranges = this._register(new DisposableStore());
	private trackedRanges: readonly TrackedRange[] = [];
	private active = false;
	private activationScheduled = false;
	private request: AbortController | undefined;
	private wordPattern: RegExp | undefined;

	constructor(private readonly view: EditorView, private readonly input: HTMLElement, private readonly viewport: View, private readonly selections: CursorsController, private readonly service: LinkedEditingService, private readonly languageId: string, private readonly defaultWordPattern: () => RegExp | undefined, private readonly onError: (error: unknown) => void = error => console.error("Stanza linked editing failed", error)) {
		super();
		if (viewport.textModel !== selections.textModel || service.textModel !== selections.textModel) throw new TypeError("Stanza linked editing dependencies must share a text model");
		this._register(addDisposableListener(input, "keydown", event => { if (event.defaultPrevented || event.isComposing || !event.shiftKey || (!event.ctrlKey && !event.metaKey) || event.altKey || event.key.toLowerCase() !== "l") return; stopEvent(event); void this.activate(); }, true));
		this._register(addDisposableListener(input, "keydown", event => { if (event.key !== "Escape" || !this.active) return; stopEvent(event); this.clear(); }, true));
		this._register(view.registerCommandTransformer(command => this.extendCommand(command)));
		this._register(selections.onDidChange(() => this.scheduleActivation()));
		this.scheduleActivation();
	}

	private async activate(): Promise<void> {
		if (this.isDisposed) return;
		this.request?.abort();
		const request = this.request = new AbortController();
		const primary = this.selections.selections.primary;
		if (!primary.isEmpty()) { this.clear(); return; }
		try {
			const result = await this.service.provideLinkedEditingRanges(this.languageId, primary.getPosition(), request.signal);
			if (request.signal.aborted || this.isDisposed) return;
			if (!result || result.ranges.length < 2) { this.clear(); return; }
			const expectedText = this.viewport.textModel.getTextInRange(result.ranges[0]!);
			if (result.ranges.some(range => this.viewport.textModel.getTextInRange(range) !== expectedText)) { this.clear(); return; }
			this.ranges.clear();
			this.trackedRanges = result.ranges.map(range => {
				const trackedRange = this.viewport.textModel.trackRange(range, TrackedRangeStickiness.NeverGrowsWhenTypingAtEdges);
				this.ranges.add(toDisposable(() => trackedRange.dispose()));
				return trackedRange;
			});
			this.wordPattern = result.wordPattern ?? this.defaultWordPattern();
			this.active = true;
			this.viewport.element.classList.add("linked-editing-active");
			this.viewport.announceAccessibilityStatus(`${result.ranges.length} linked editing ranges active`);
		} catch (error) { if (!request.signal.aborted) this.onError(error); }
	}

	private extendCommand(command: EditorEditCommand): EditorEditCommand {
		if (!this.active || command.edits.length !== 1) return command;
		const sourceEdit = command.edits[0]!;
		const sourceEditRange = Range.lift(sourceEdit.range);
		const source = this.trackedRanges.find(candidate => containsRange(candidate.range, sourceEditRange));
		if (!source) return command;
		const model = this.viewport.textModel;
		const sourceStartOffset = model.offsetAt(source.range.getStartPosition());
		const relativeStartOffset = model.offsetAt(sourceEditRange.getStartPosition()) - sourceStartOffset;
		const relativeEndOffset = model.offsetAt(sourceEditRange.getEndPosition()) - sourceStartOffset;
		const currentValue = model.getTextInRange(source.range);
		const nextValue = currentValue.slice(0, relativeStartOffset) + sourceEdit.text + currentValue.slice(relativeEndOffset);
		if (this.wordPattern && !matchesEntirePattern(this.wordPattern, nextValue)) {
			this.clear();
			return command;
		}
		const edits = this.trackedRanges
			.filter(candidate => candidate !== source)
			.map(candidate => {
				const targetStartOffset = model.offsetAt(candidate.range.getStartPosition());
				const targetEndOffset = model.offsetAt(candidate.range.getEndPosition());
				const startOffset = targetStartOffset + relativeStartOffset;
				const endOffset = targetStartOffset + relativeEndOffset;
				if (startOffset < targetStartOffset || endOffset > targetEndOffset) return undefined;
				return { range: Range.fromPositions(model.positionAt(startOffset), model.positionAt(endOffset)), text: sourceEdit.text };
			})
			.filter((edit): edit is { readonly range: Range; readonly text: string } => edit !== undefined)
			.sort((left, right) => Position.compare(left.range.getStartPosition(), right.range.getStartPosition()));
		return extendEditorEditCommand(model, command, edits);
	}

	private scheduleActivation(): void {
		if (this.activationScheduled || this.isDisposed) return;
		this.activationScheduled = true;
		queueMicrotask(() => {
			this.activationScheduled = false;
			if (this.isDisposed) return;
			void this.activate();
		});
	}

	private clear(): void {
		this.request?.abort();
		this.request = undefined;
		this.ranges.clear();
		this.trackedRanges = [];
		this.wordPattern = undefined;
		this.active = false;
		this.viewport.element.classList.remove("linked-editing-active");
	}
}

function containsRange(container: Range, candidate: Range): boolean {
	return Position.compare(container.getStartPosition(), candidate.getStartPosition()) <= 0 && Position.compare(container.getEndPosition(), candidate.getEndPosition()) >= 0;
}

function matchesEntirePattern(pattern: RegExp, value: string): boolean {
	pattern.lastIndex = 0;
	const match = pattern.exec(value);
	pattern.lastIndex = 0;
	return match?.index === 0 && match[0].length === value.length;
}

registerTextEditorCapabilityContribution({ id: "editor.contrib.linkedEditing", install: context => {
	if (context.kind !== "text") return;
	const service = context.register(new LinkedEditingService(context.model, context.languageFeaturesService.linkedEditingProvider, context.options.input.resource));
	context.register(new LinkedEditingController(context.view, context.view.element, context.viewport, context.selections, service, context.languageId, () => context.configurations.getLanguageConfiguration(context.languageId).wordPattern, context.onLanguageError));
} });
