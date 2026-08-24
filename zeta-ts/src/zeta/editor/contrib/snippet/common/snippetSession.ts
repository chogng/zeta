import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { EditorCommandHistoryMode, type EditorEditCommand } from "../../../common/commands/editorEditCommand.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { type LanguageCompletionSnippet } from "./snippetParser.js";
import { applyLanguageCompletionSnippetTransform, type LanguageCompletionSnippetTransform } from "./snippetTransform.js";
import { TextSelection, TextSelectionSet } from "../../../common/core/selection.js";
import { TextRange } from "../../../common/core/text.js";
import { type TextModel } from "../../../common/model/textModel.js";
import { TrackedRangeStickiness, type TrackedRange } from "../../../common/model/trackedRange.js";

/**
 * Owns one accepted completion snippet's tabstop navigation.
 *
 * Every tabstop occurrence is tracked through later model transactions. The
 * session owns neither the model nor the editor selection controller, and may
 * be disposed without changing inserted text.
 */
export class LanguageCompletionSnippetSession extends DisposableOwner {
	private readonly groups: readonly SnippetTrackedGroup[];
	private readonly transforms: readonly SnippetTrackedTransform[];
	private readonly choiceIndexes = new Map<number, number>();
	private readonly finalRange: TrackedRange;
	private currentGroupIndex = 0;

	constructor(
		model: TextModel,
		private readonly selections: EditorSelectionController,
		insertionStartOffset: number,
		snippet: LanguageCompletionSnippet,
		finalOffsetWithinInsertion = snippet.text.length,
	) {
		super();
		if (model !== selections.textModel) {
			this.dispose();
			throw new TypeError("Language completion snippet session must share its editor text model");
		}
		if (!Number.isSafeInteger(insertionStartOffset) || insertionStartOffset < 0 || insertionStartOffset > model.getText().length) {
			this.dispose();
			throw new RangeError("Language completion snippet insertion offset is outside its text model");
		}
		if (!snippet || typeof snippet.text !== "string" || snippet.placeholderGroups.length === 0) {
			this.dispose();
			throw new TypeError("Language completion snippet session requires at least one parsed tabstop");
		}
		if (!Number.isSafeInteger(finalOffsetWithinInsertion) || finalOffsetWithinInsertion < snippet.text.length) {
			this.dispose();
			throw new RangeError("Language completion snippet final offset must follow its expansion text");
		}
		try {
			this.groups = Object.freeze(snippet.placeholderGroups.map(group => Object.freeze({
				index: group.index,
				ranges: Object.freeze(group.placeholders.map(placeholder =>
					model.trackRange(
						TextRange.from(
							model.positionAt(insertionStartOffset + placeholder.startOffset),
							model.positionAt(insertionStartOffset + placeholder.endOffset),
						),
						TrackedRangeStickiness.NeverGrowsAtEdges,
					)
				)),
				...(group.choices ? { choices: group.choices } : {}),
			})));
			this.transforms = Object.freeze((snippet.transforms ?? []).map(transform => Object.freeze({
				index: transform.index,
				transform: transform.transform,
				range: model.trackRange(
					TextRange.from(
						model.positionAt(insertionStartOffset + transform.startOffset),
						model.positionAt(insertionStartOffset + transform.endOffset),
					),
					TrackedRangeStickiness.NeverGrowsAtEdges,
				),
			})));
			this.finalRange = model.trackRange(
				TextRange.emptyAt(model.positionAt(insertionStartOffset + finalOffsetWithinInsertion)),
				TrackedRangeStickiness.NeverGrowsAtEdges,
			);
			this.defer(() => {
				for (const group of this.groups) {
					for (const range of group.ranges) range.dispose();
				}
				for (const transform of this.transforms) transform.range.dispose();
				this.finalRange.dispose();
			});
		} catch (error) {
			this.dispose();
			throw error;
		}
	}

	/** Whether this session has released its tracked tabstops. */
	get isDisposed(): boolean { return super.isDisposed; }

	/** Advances to the next tabstop or consumes the final Tab that leaves the snippet. */
	selectNext(): boolean {
		this.assertNotDisposed();
		if (this.currentGroupIndex + 1 < this.groups.length) {
			this.synchronizeTransforms(this.groups[this.currentGroupIndex]!);
			this.currentGroupIndex += 1;
			this.selectGroup(this.currentGroupIndex);
			return true;
		}
		if (this.currentGroupIndex === this.groups.length - 1) {
			this.synchronizeTransforms(this.groups[this.currentGroupIndex]!);
			this.currentGroupIndex += 1;
			this.selections.setSelections(TextSelectionSet.single(TextSelection.collapsedAt(this.finalRange.range.end)));
			return true;
		}
		this.dispose();
		return true;
	}

	/** Moves to the preceding tabstop; no selection changes occur before the first group. */
	selectPrevious(): boolean {
		this.assertNotDisposed();
		if (this.currentGroupIndex === this.groups.length) {
			this.currentGroupIndex -= 1;
			this.selectGroup(this.currentGroupIndex);
			return true;
		}
		if (this.currentGroupIndex === 0) return false;
		this.synchronizeTransforms(this.groups[this.currentGroupIndex]!);
		this.currentGroupIndex -= 1;
		this.selectGroup(this.currentGroupIndex);
		return true;
	}

	/** Replaces the active choice tabstop and every mirrored occurrence with its next value. */
	selectNextChoice(): boolean {
		return this.selectRelativeChoice(1);
	}

	/** Replaces the active choice tabstop and every mirrored occurrence with its previous value. */
	selectPreviousChoice(): boolean {
		return this.selectRelativeChoice(-1);
	}

	/** Leaves navigation active text unchanged and releases its tracked tabstops. */
	cancel(): void {
		this.assertNotDisposed();
		this.dispose();
	}

	private selectGroup(index: number): void {
		const group = this.groups[index];
		if (!group) throw new RangeError("Language completion snippet tabstop index is outside its session");
		this.selections.setSelections(TextSelectionSet.withPrimary(
			group.ranges.map(range => TextSelection.from(range.range.start, range.range.end)),
			0,
		));
	}

	private selectRelativeChoice(delta: number): boolean {
		this.assertNotDisposed();
		const group = this.groups[this.currentGroupIndex];
		if (!group?.choices || group.choices.length === 0) return false;
		const current = this.choiceIndexes.get(this.currentGroupIndex) ?? 0;
		const next = (current + delta + group.choices.length) % group.choices.length;
		if (!this.replaceChoice(group, group.choices[next]!)) return false;
		this.choiceIndexes.set(this.currentGroupIndex, next);
		this.synchronizeTransforms(group);
		return true;
	}

	private replaceChoice(group: SnippetTrackedGroup, text: string): boolean {
		const model = this.selections.textModel;
		const ranges = group.ranges.map(range => ({
			range,
			startOffset: model.offsetAt(range.range.start),
			endOffset: model.offsetAt(range.range.end),
		})).sort((left, right) => left.startOffset - right.startOffset || left.endOffset - right.endOffset);
		for (let index = 1; index < ranges.length; index += 1) {
			const previous = ranges[index - 1]!;
			const current = ranges[index]!;
			if (current.startOffset < previous.endOffset) return false;
		}
		let cumulativeDelta = 0;
		const selectionsAfter = new Map<TrackedRange, { readonly anchorOffset: number; readonly activeOffset: number }>();
		for (const range of ranges) {
			const startOffset = range.startOffset + cumulativeDelta;
			const endOffset = startOffset + text.length;
			selectionsAfter.set(range.range, { anchorOffset: startOffset, activeOffset: endOffset });
			cumulativeDelta += text.length - (range.endOffset - range.startOffset);
		}
		const command: EditorEditCommand = Object.freeze({
			edits: Object.freeze(ranges.map(range => Object.freeze({
				range: range.range.range,
				text,
			}))),
			selectionsAfter: Object.freeze(group.ranges.map(range => {
				const selection = selectionsAfter.get(range);
				if (!selection) throw new Error("Language completion choice selection is missing");
				return Object.freeze(selection);
			})),
			primarySelectionIndex: 0,
			historyMode: EditorCommandHistoryMode.Isolated,
		});
		return this.selections.execute(command) !== undefined;
	}

	private synchronizeTransforms(group: SnippetTrackedGroup): void {
		const transforms = this.transforms.filter(transform => transform.index === group.index);
		if (transforms.length === 0) return;
		const sourceRange = group.ranges[0];
		if (!sourceRange) return;
		const model = this.selections.textModel;
		const sourceText = model.getTextInRange(sourceRange.range);
		const edits = transforms.flatMap(transform => {
			const text = applyLanguageCompletionSnippetTransform(sourceText, transform.transform);
			return model.getTextInRange(transform.range.range) === text ? [] : [{ range: transform.range.range, text }];
		});
		if (edits.length > 0) model.applyEdits(edits);
	}

}

interface SnippetTrackedGroup {
	readonly index: number;
	readonly ranges: readonly TrackedRange[];
	readonly choices?: readonly string[];
}

interface SnippetTrackedTransform {
	readonly index: number;
	readonly transform: LanguageCompletionSnippetTransform;
	readonly range: TrackedRange;
}
