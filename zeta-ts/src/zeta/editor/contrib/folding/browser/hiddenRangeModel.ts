import { Emitter, type Event } from "../../../../base/common/event.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type EditorLineVisibilitySource } from "../../../common/viewModel/modelLineProjection.js";
import { type TextModel } from "../../../common/model/textModel.js";
import { EditorFoldingModel } from "./foldingModel.js";

/** Derives hidden physical lines from collapsed folding regions for visual consumers. */
export class EditorHiddenRangeModel extends DisposableOwner implements EditorLineVisibilitySource {
	private readonly changeEmitter = this.own(new Emitter<void>());
	private hiddenLines: readonly boolean[] = Object.freeze([]);
	private visibleLineIndexes: readonly number[] = Object.freeze([]);

	readonly onDidChange: Event<void> = this.changeEmitter.event;

	constructor(private readonly textModel: TextModel, private readonly folding: EditorFoldingModel) {
		super();
		if (folding.model !== textModel) throw new TypeError("Hidden range and folding models must share one text model");
		this.rebuild();
		this.own(folding.onDidChange(() => this.rebuild()));
		this.own(textModel.onDidChange(() => this.rebuild()));
	}

	get model(): TextModel {
		return this.textModel;
	}

	get lineCount(): number {
		return this.textModel.lineCount;
	}

	isLineVisible(lineIndex: number): boolean {
		validateLineIndex(this.textModel, lineIndex);
		return !this.hiddenLines[lineIndex];
	}

	isLineHidden(lineIndex: number): boolean {
		return !this.isLineVisible(lineIndex);
	}

	getVisibleLineIndexes(): readonly number[] {
		return this.visibleLineIndexes;
	}

	private rebuild(): void {
		const hiddenLines = Array.from({ length: this.textModel.lineCount }, () => false);
		for (const region of this.folding.regions) {
			if (!region.collapsed) continue;
			for (let lineIndex = region.startLineIndex + 1; lineIndex <= region.endLineIndex; lineIndex += 1) hiddenLines[lineIndex] = true;
		}
		const visibleLineIndexes = hiddenLines.flatMap((hidden, lineIndex) => hidden ? [] : [lineIndex]);
		if (sameBooleanArray(this.hiddenLines, hiddenLines)) return;
		this.hiddenLines = Object.freeze(hiddenLines);
		this.visibleLineIndexes = Object.freeze(visibleLineIndexes);
		this.changeEmitter.fire();
	}
}

function sameBooleanArray(left: readonly boolean[], right: readonly boolean[]): boolean {
	return left.length === right.length && left.every((value, index) => value === right[index]);
}

function validateLineIndex(model: TextModel, lineIndex: number): void {
	if (!Number.isSafeInteger(lineIndex) || lineIndex < 0 || lineIndex >= model.lineCount) throw new RangeError("Hidden line index is outside the text model");
}
