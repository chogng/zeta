import { SelectionDirection, TextSelection } from '../core/selection.js';
import { AbstractDisposable } from '../../../base/common/lifecycle.js';
import { type TextModel } from '../model/textModel.js';
import { TrackedRangeStickiness, type TrackedRange } from '../model/trackedRange.js';

export class Cursor extends AbstractDisposable {
	private readonly trackedRange: TrackedRange;

	constructor(model: TextModel, selection: TextSelection) {
		super();
		this.trackedRange = model.trackRange(selection.range, TrackedRangeStickiness.NeverGrowsAtEdges);
		this.direction = selection.direction;
	}

	public readonly direction: SelectionDirection;

	public get selection(): TextSelection {
		const range = this.trackedRange.range;
		return this.direction === SelectionDirection.Backward
			? TextSelection.from(range.end, range.start)
			: TextSelection.from(range.start, range.end);
	}

	protected override disposeCore(): void {
		this.trackedRange.dispose();
	}
}
