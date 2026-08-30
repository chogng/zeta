import { type Range } from '../core/range.js';

/** Decoration kinds understood by the common line renderer. */
export enum InlineDecorationType {
	Regular = 0,
	Before = 1,
	After = 2,
	RegularAffectingLetterSpacing = 3,
	WidthOnly = 4,
}

export class InlineDecoration {
	constructor(
		public readonly range: Range,
		public readonly inlineClassName: string,
		public readonly type: InlineDecorationType,
	) { }
}
