import { isNonNegativeSafeInteger } from '../../../base/common/numbers.js';
import { Selection } from '../core/selection.js';

/**
 * Zeta-owned immutable multi-selection state with one explicit primary item.
 */
export class SelectionSet {
	private constructor(
		readonly selections: readonly Selection[],
		readonly primaryIndex: number,
	) {
		Object.freeze(this);
	}

	static single(selection: Selection): SelectionSet {
		return new SelectionSet(Object.freeze([selection]), 0);
	}

	static withPrimary(
		selections: readonly Selection[],
		primaryIndex: number,
	): SelectionSet {
		if (selections.length === 0) {
			throw new RangeError('SelectionSet must not be empty');
		}
		if (!isNonNegativeSafeInteger(primaryIndex) || primaryIndex >= selections.length) {
			throw new RangeError(`primaryIndex must be between 0 and ${selections.length - 1}`);
		}
		return new SelectionSet(Object.freeze([...selections]), primaryIndex);
	}

	get primary(): Selection {
		return this.selections[this.primaryIndex];
	}

	equals(other: SelectionSet): boolean {
		return this.primaryIndex === other.primaryIndex
			&& this.selections.length === other.selections.length
			&& this.selections.every((selection, index) => selection.equalsSelection(other.selections[index]!));
	}

	map(mapper: (selection: Selection, index: number) => Selection): SelectionSet {
		return SelectionSet.withPrimary(this.selections.map(mapper), this.primaryIndex);
	}
}
