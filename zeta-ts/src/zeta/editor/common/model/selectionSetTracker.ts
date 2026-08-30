import { AbstractDisposable, Disposable, DisposableStore } from '../../../base/common/lifecycle.js';
import { SelectionDirection, Selection } from '../core/selection.js';
import { TrackedRangeStickiness } from '../model.js';
import { SelectionSet } from '../cursor/selectionSet.js';
import { type TextModel } from './textModel.js';
import { type TrackedRange } from './trackedRange.js';

/** Tracks Zeta's explicit-primary `SelectionSet` through text-model edits. */
export class SelectionSetTracker extends Disposable {
	private readonly resources = this._register(new DisposableStore());
	private trackedSelections: TrackedSelection[] = [];
	private primaryIndex = 0;

	constructor(private readonly model: TextModel, selections: SelectionSet) {
		super();
		this.setSelections(selections);
	}

	getSelections(): SelectionSet {
		return SelectionSet.withPrimary(this.trackedSelections.map(selection => selection.selection), this.primaryIndex);
	}

	setSelections(selections: SelectionSet): void {
		validateSelectionSet(this.model, selections);
		this.resources.clear();
		this.trackedSelections = selections.selections.map(selection => {
			const trackedSelection = new TrackedSelection(this.model, selection);
			this.resources.add(trackedSelection);
			return trackedSelection;
		});
		this.primaryIndex = selections.primaryIndex;
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

export function validateSelectionSet(model: TextModel, selections: SelectionSet): void {
	for (const selection of selections.selections) {
		model.offsetAt(selection.getSelectionStart());
		model.offsetAt(selection.getPosition());
	}
}
