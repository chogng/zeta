import { type TextRange } from '../core/text.js';

/** Decoration kinds understood by the common line renderer. */
export enum InlineDecorationType {
	Regular = 'regular',
	RegularAffectingLetterSpacing = 'regularAffectingLetterSpacing',
	Before = 'before',
	After = 'after',
}

/** Common decoration input kept independent from browser CSS and DOM. */
export interface InlineDecoration {
	readonly range: TextRange;
	readonly inlineClassName: string;
	readonly type: InlineDecorationType;
}
