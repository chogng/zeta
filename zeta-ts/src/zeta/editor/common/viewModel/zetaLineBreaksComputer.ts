import { WrappingIndent } from '../config/editorOptions.js';

/** Zeta's current per-line pixel measurement contract for soft wrapping. */
export interface ZetaLineBreaksComputer {
	computeLineBreaks(text: string, wrapWidth: number, wrappingIndent?: WrappingIndent): readonly number[];
	computeLineBreaksWithIndent?(text: string, wrapWidth: number, wrappingIndent: WrappingIndent): ZetaLineBreaksResult;
}

export interface ZetaLineBreaksResult {
	readonly breakColumns: readonly number[];
	readonly wrappedTextIndentWidth: number;
}
