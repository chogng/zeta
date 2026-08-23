import { IPosition, TextPosition } from "./position.js";
import { TextRange } from "./range.js";

export interface ISelection {
	readonly anchor: IPosition;
	readonly active: IPosition;
}

export enum SelectionDirection {
	Forward = "forward",
	Backward = "backward",
}

/**
 * One immutable selection with an anchor and an active cursor position.
 */
export class TextSelection {
	private readonly orderedRange: TextRange;

	private constructor(
		readonly anchor: TextPosition,
		readonly active: TextPosition,
	) {
		this.orderedRange = anchor.compareTo(active) <= 0
			? TextRange.from(anchor, active)
			: TextRange.from(active, anchor);
		Object.freeze(this);
	}

	static from(
		anchor: IPosition,
		active: IPosition,
	): TextSelection {
		return new TextSelection(TextPosition.lift(anchor), TextPosition.lift(active));
	}

	static collapsedAt(position: IPosition): TextSelection {
		const lifted = TextPosition.lift(position);
		return new TextSelection(lifted, lifted);
	}

	static fromRange(range: TextRange, direction: SelectionDirection): TextSelection {
		return direction === SelectionDirection.Forward
			? new TextSelection(range.start, range.end)
			: new TextSelection(range.end, range.start);
	}

	get range(): TextRange {
		return this.orderedRange;
	}

	get direction(): SelectionDirection {
		return this.anchor.compareTo(this.active) <= 0
			? SelectionDirection.Forward
			: SelectionDirection.Backward;
	}

	get collapsed(): boolean {
		return this.anchor.compareTo(this.active) === 0;
	}

	static selectionsEqual(left: ISelection, right: ISelection): boolean { return TextPosition.equals(left.anchor, right.anchor) && TextPosition.equals(left.active, right.active); }
	static selectionsArrEqual(left: readonly ISelection[] | undefined, right: readonly ISelection[] | undefined): boolean { return left === right || Boolean(left && right && left.length === right.length && left.every((selection, index) => TextSelection.selectionsEqual(selection, right[index]!))); }
	static liftSelection(selection: ISelection): TextSelection { return TextSelection.from(selection.anchor, selection.active); }
	static isISelection(value: unknown): value is ISelection { return Boolean(value && typeof value === "object" && TextPosition.isIPosition((value as ISelection).anchor) && TextPosition.isIPosition((value as ISelection).active)); }
	static createWithDirection(start: IPosition, end: IPosition, direction: SelectionDirection): TextSelection { return TextSelection.fromRange(TextRange.from(start, end), direction); }
	equals(other: ISelection): boolean { return TextSelection.selectionsEqual(this, other); }
	equalsSelection(other: ISelection): boolean { return this.equals(other); }
	getPosition(): TextPosition { return this.active; }
	getSelectionStart(): TextPosition { return this.range.start; }
	setAnchor(anchor: IPosition): TextSelection { return new TextSelection(TextPosition.lift(anchor), this.active); }
	setActive(active: IPosition): TextSelection { return new TextSelection(this.anchor, TextPosition.lift(active)); }
	setStartPosition(position: IPosition): TextSelection { return this.direction === SelectionDirection.Forward ? new TextSelection(TextPosition.lift(position), this.active) : new TextSelection(this.anchor, TextPosition.lift(position)); }
	setEndPosition(position: IPosition): TextSelection { return this.direction === SelectionDirection.Forward ? new TextSelection(this.anchor, TextPosition.lift(position)) : new TextSelection(TextPosition.lift(position), this.active); }
	toString(): string { return `[${this.anchor.toString()} -> ${this.active.toString()}]`; }
}

/**
 * An immutable non-empty multi-selection set with one explicit primary item.
 */
export class TextSelectionSet {
	private constructor(
		readonly selections: readonly TextSelection[],
		readonly primaryIndex: number,
	) {
		Object.freeze(this);
	}

	static single(selection: TextSelection): TextSelectionSet {
		return new TextSelectionSet(Object.freeze([selection]), 0);
	}

	static withPrimary(
		selections: readonly TextSelection[],
		primaryIndex: number,
	): TextSelectionSet {
		if (selections.length === 0) {
			throw new RangeError("TextSelectionSet must not be empty");
		}
		if (
			!Number.isSafeInteger(primaryIndex) ||
			primaryIndex < 0 ||
			primaryIndex >= selections.length
		) {
			throw new RangeError(
				`primaryIndex must be between 0 and ${selections.length - 1}`,
			);
		}
		return new TextSelectionSet(
			Object.freeze([...selections]),
			primaryIndex,
		);
	}

	get primary(): TextSelection {
		return this.selections[this.primaryIndex];
	}

	equals(other: TextSelectionSet): boolean {
		return this.primaryIndex === other.primaryIndex && this.selections.length === other.selections.length && this.selections.every((selection, index) => selection.equals(other.selections[index]!));
	}

	map(mapper: (selection: TextSelection, index: number) => TextSelection): TextSelectionSet {
		return TextSelectionSet.withPrimary(this.selections.map(mapper), this.primaryIndex);
	}
}
