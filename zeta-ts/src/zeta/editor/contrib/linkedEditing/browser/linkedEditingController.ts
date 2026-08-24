import "./media/linkedEditing.css";
import { registerEditorContribution } from "../../../browser/editorContribution.js";
import { type TextInputController } from "../../../browser/input/textInputController.js";
import { type EditorViewport } from "../../../browser/view/editorViewport.js";
import { addDisposableListener, stopEvent } from "../../../../base/browser/dom.js";
import { DisposableOwner, ResettableDisposableGroup } from "../../../../base/common/lifecycle.js";
import { extendEditorEditCommand } from "../../../common/commands/editorCommand.js";
import { type EditorEditCommand } from "../../../common/commands/editorEditCommand.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { TextRange, type TextEdit } from "../../../common/core/text.js";
import { TrackedRangeStickiness, type TrackedRange } from "../../../common/model/trackedRange.js";
import { type LinkedEditingService } from "../common/linkedEditing.js";

/** Synchronizes provider-declared linked ranges through one atomic native-input transaction. */
export class LinkedEditingController extends DisposableOwner {
	private readonly ranges = this.own(new ResettableDisposableGroup());
	private trackedRanges: readonly TrackedRange[] = [];
	private active = false;
	private activationScheduled = false;
	private request: AbortController | undefined;
	private wordPattern: RegExp | undefined;

	constructor(private readonly inputController: TextInputController, private readonly input: HTMLTextAreaElement, private readonly viewport: EditorViewport, private readonly selections: EditorSelectionController, private readonly service: LinkedEditingService, private readonly languageId: string, private readonly defaultWordPattern: () => RegExp | undefined, private readonly onError: (error: unknown) => void = error => console.error("Stanza linked editing failed", error)) {
		super();
		if (viewport.textModel !== selections.textModel || service.textModel !== selections.textModel) throw new TypeError("Stanza linked editing dependencies must share a text model");
		this.own(addDisposableListener(input, "keydown", event => { if (event.defaultPrevented || event.isComposing || !event.shiftKey || (!event.ctrlKey && !event.metaKey) || event.altKey || event.key.toLowerCase() !== "l") return; stopEvent(event); void this.activate(); }, true));
		this.own(addDisposableListener(input, "keydown", event => { if (event.key !== "Escape" || !this.active) return; stopEvent(event); this.clear(); }, true));
		this.own(inputController.registerCommandTransformer(command => this.extendCommand(command)));
		this.own(selections.onDidChange(() => this.scheduleActivation()));
		this.scheduleActivation();
	}

	private async activate(): Promise<void> {
		if (this.isDisposed) return;
		this.request?.abort();
		const request = this.request = new AbortController();
		const primary = this.selections.selections.primary;
		if (!primary.collapsed) { this.clear(); return; }
		try {
			const result = await this.service.provideLinkedEditingRanges(this.languageId, primary.active, request.signal);
			if (request.signal.aborted || this.isDisposed) return;
			if (!result || result.ranges.length < 2) { this.clear(); return; }
			const expectedText = this.viewport.textModel.getTextInRange(result.ranges[0]!);
			if (result.ranges.some(range => this.viewport.textModel.getTextInRange(range) !== expectedText)) { this.clear(); return; }
			this.ranges.clear();
			this.trackedRanges = result.ranges.map(range => this.ranges.adopt(this.viewport.textModel.trackRange(range, TrackedRangeStickiness.NeverGrowsAtEdges), candidate => candidate.dispose()));
			this.wordPattern = result.wordPattern ?? this.defaultWordPattern();
			this.active = true;
			this.viewport.element.classList.add("linked-editing-active");
			this.viewport.announceAccessibilityStatus(`${result.ranges.length} linked editing ranges active`);
		} catch (error) { if (!request.signal.aborted) this.onError(error); }
	}

	private extendCommand(command: EditorEditCommand): EditorEditCommand {
		if (!this.active || command.edits.length !== 1) return command;
		const sourceEdit = command.edits[0]!;
		const source = this.trackedRanges.find(candidate => containsRange(candidate.range, sourceEdit.range));
		if (!source) return command;
		const model = this.viewport.textModel;
		const sourceStartOffset = model.offsetAt(source.range.start);
		const relativeStartOffset = model.offsetAt(sourceEdit.range.start) - sourceStartOffset;
		const relativeEndOffset = model.offsetAt(sourceEdit.range.end) - sourceStartOffset;
		const currentValue = model.getTextInRange(source.range);
		const nextValue = currentValue.slice(0, relativeStartOffset) + sourceEdit.text + currentValue.slice(relativeEndOffset);
		if (this.wordPattern && !matchesEntirePattern(this.wordPattern, nextValue)) {
			this.clear();
			return command;
		}
		const edits = this.trackedRanges
			.filter(candidate => candidate !== source)
			.map(candidate => {
				const targetStartOffset = model.offsetAt(candidate.range.start);
				const targetEndOffset = model.offsetAt(candidate.range.end);
				const startOffset = targetStartOffset + relativeStartOffset;
				const endOffset = targetStartOffset + relativeEndOffset;
				if (startOffset < targetStartOffset || endOffset > targetEndOffset) return undefined;
				return { range: TextRange.from(model.positionAt(startOffset), model.positionAt(endOffset)), text: sourceEdit.text };
			})
			.filter((edit): edit is TextEdit => edit !== undefined)
			.sort((left, right) => left.range.start.compareTo(right.range.start));
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

function containsRange(container: TextRange, candidate: TextRange): boolean {
	return container.start.compareTo(candidate.start) <= 0 && container.end.compareTo(candidate.end) >= 0;
}

function matchesEntirePattern(pattern: RegExp, value: string): boolean {
	pattern.lastIndex = 0;
	const match = pattern.exec(value);
	pattern.lastIndex = 0;
	return match?.index === 0 && match[0].length === value.length;
}

registerEditorContribution({ id: "editor.contrib.linkedEditing", install: context => {
	if (context.kind !== "text") return;
	const service = context.own(context.languageFeaturesService.createLinkedEditingService(context.model, context.options.input.resource));
	context.own(new LinkedEditingController(context.textInput, context.textInput.element, context.viewport, context.selections, service, context.languageId, () => context.configurations.getLanguageConfiguration(context.languageId).wordPattern, context.onLanguageError));
} });
