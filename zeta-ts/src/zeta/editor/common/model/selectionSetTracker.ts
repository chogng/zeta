import { AbstractDisposable, Disposable, DisposableStore } from '../../../base/common/lifecycle.js';
import { SelectionDirection, Selection } from '../core/selection.js';
import { TrackedRangeStickiness } from '../model.js';
import { type TextModel } from './textModel.js';
import { type TrackedRange } from './trackedRange.js';

/** Tracks one primary-first selection array through text-model edits. */
export class SelectionSetTracker extends Disposable {
	private readonly resources = this._register(new DisposableStore());
	private trackedSelections: TrackedSelection[] = [];

	constructor(private readonly model: TextModel, selections: readonly Selection[]) {
		super();
		this.setSelections(selections);
	}

	getSelections(): readonly Selection[] {
		return Object.freeze(this.trackedSelections.map(selection => selection.selection));
	}

	setSelections(selections: readonly Selection[]): void {
		validateSelections(this.model, selections);
		this.resources.clear();
		this.trackedSelections = selections.map(selection => {
			const trackedSelection = new TrackedSelection(this.model, selection);
			this.resources.add(trackedSelection);
			return trackedSelection;
		});
	}
}

class TrackedSelection extends AbstractDisposable {
	private readonly trackedRange: TrackedRange;
	private readonly direction: SelectionDirection;

	constructor(model: TextModel, selection: Selection) {
		super();
		this.trackedRange = model.trackRange(selection, TrackedRangeStickiness.NeverGrowsWhenTypingAtEdges);
		this.direction = selection.getDirection();
	}

	get selection(): Selection {
		const range = this.trackedRange.range;
		return this.direction === SelectionDirection.RTL
			? Selection.fromPositions(range.getEndPosition(), range.getStartPosition())
			: Selection.fromPositions(range.getStartPosition(), range.getEndPosition());
	}

	protected override disposeCore(): void {
		this.trackedRange.dispose();
	}
}

export function validateSelections(model: TextModel, selections: readonly Selection[]): void {
	if (selections.length === 0) throw new RangeError('Selections must not be empty');
	for (const selection of selections) {
		model.offsetAt(selection.getSelectionStart());
		model.offsetAt(selection.getPosition());
	}
}
