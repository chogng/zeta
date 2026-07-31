import { TextPosition, TextRange } from "./text.js";

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
    anchor: TextPosition,
    active: TextPosition,
  ): TextSelection {
    return new TextSelection(anchor, active);
  }

  static collapsedAt(position: TextPosition): TextSelection {
    return new TextSelection(position, position);
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
}
