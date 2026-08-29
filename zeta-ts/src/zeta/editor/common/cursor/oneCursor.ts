import { SelectionDirection, Selection } from '../core/selection.js';
import { AbstractDisposable } from '../../../base/common/lifecycle.js';
import { type TextModel } from '../model/textModel.js';
import { TrackedRangeStickiness, type TrackedRange } from '../model/trackedRange.js';

export class Cursor extends AbstractDisposable {
	private readonly trackedRange: TrackedRange;

	constructor(model: TextModel, selection: Selection) {
		super();
		this.trackedRange = model.trackRange(selection, TrackedRangeStickiness.NeverGrowsAtEdges);
		this.direction = selection.getDirection();
	}

	public readonly direction: SelectionDirection;

	public get selection(): Selection {
		const range = this.trackedRange.range;
		return this.direction === SelectionDirection.RTL
			? Selection.fromPositions(range.getEndPosition(), range.getStartPosition())
			: Selection.fromPositions(range.getStartPosition(), range.getEndPosition());
	}

	protected override disposeCore(): void {
		this.trackedRange.dispose();
	}
}
