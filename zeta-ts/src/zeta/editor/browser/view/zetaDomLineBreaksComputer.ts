import { isFiniteNumber } from '../../../base/common/numbers.js';
import { isWrappingIndent, WrappingIndent } from '../../common/config/editorOptions.js';
import { getTextGraphemeBoundaries } from '../../common/core/textSegmentation.js';
import { type TextMeasurer } from '../../common/viewModel/textMeasurer.js';
import { type ZetaLineBreaksComputer, type ZetaLineBreaksResult } from '../../common/viewModel/zetaLineBreaksComputer.js';

/**
 * Browser-side line-break calculation used by the common view-model lines.
 *
 * The projection owns model-versioned rows; this class owns the browser font
 * measurement policy needed to turn one logical line into visual fragments.
 */
export class ZetaDOMLineBreaksComputer implements ZetaLineBreaksComputer {
	public constructor(
		private readonly textMeasurer: TextMeasurer,
		private readonly tabSize = 4,
	) {
		if (!Number.isSafeInteger(tabSize) || tabSize < 1) {
			throw new RangeError('Stanza DOM line-break tab size must be a positive safe integer');
		}
	}

	public computeLineBreaks(text: string, wrapWidth: number, wrappingIndent = WrappingIndent.Same): readonly number[] {
		return this.computeLineBreaksWithIndent(text, wrapWidth, wrappingIndent).breakColumns;
	}

	public computeLineBreaksWithIndent(text: string, wrapWidth: number, wrappingIndent: WrappingIndent): ZetaLineBreaksResult {
		if (!isFiniteNumber(wrapWidth) || wrapWidth < 0) {
			throw new RangeError('Stanza DOM line-break width must be finite and non-negative');
		}
		if (!isWrappingIndent(wrappingIndent)) {
			throw new TypeError('Unknown Stanza wrapping indent mode');
		}
		if (text.length === 0 || wrapWidth === 0) {
			return Object.freeze({
				breakColumns: Object.freeze([text.length]),
				wrappedTextIndentWidth: 0,
			});
		}
		const wrappedTextIndentWidth = computeWrappedTextIndentWidth(text, wrapWidth, wrappingIndent, this.tabSize, value => this.measure(value));

		const breaks: number[] = [];
		const boundaries = getTextGraphemeBoundaries(text);
		let startColumn = 0;
		let previousColumn = 0;
		for (let index = 1; index < boundaries.length; index += 1) {
			const column = boundaries[index]!;
			const width = this.measure(text.slice(startColumn, column));
			const availableWidth = startColumn === 0
				? wrapWidth
				: Math.max(0, wrapWidth - wrappedTextIndentWidth);
			if (width > availableWidth && previousColumn > startColumn) {
				breaks.push(previousColumn);
				startColumn = previousColumn;
			}
			previousColumn = column;
		}
		breaks.push(text.length);
		return Object.freeze({
			breakColumns: Object.freeze(breaks),
			wrappedTextIndentWidth,
		});
	}

	private measure(text: string): number {
		const width = this.textMeasurer.measureLineWidth(text);
		if (!isFiniteNumber(width) || width < 0) {
			throw new RangeError('Stanza wrapped line measurement must be finite and non-negative');
		}
		return width;
	}
}

function computeWrappedTextIndentWidth(
	text: string,
	wrapWidth: number,
	wrappingIndent: WrappingIndent,
	tabSize: number,
	measure: (text: string) => number,
): number {
	if (wrappingIndent === WrappingIndent.None || text.length === 0) return 0;
	const firstNonWhitespaceIndex = findFirstNonWhitespaceIndex(text);
	if (firstNonWhitespaceIndex < 0) return 0;
	const additionalIndentLevels = wrappingIndent === WrappingIndent.DeepIndent
		? 2
		: wrappingIndent === WrappingIndent.Indent
			? 1
			: 0;
	const existingIndent = text.slice(0, firstNonWhitespaceIndex);
	if (existingIndent.length === 0 && additionalIndentLevels === 0) return 0;
	const indent = existingIndent + '\t'.repeat(additionalIndentLevels);
	const indentWidth = measure(indent);
	if (indentWidth === 0) return 0;
	const boundaries = getTextGraphemeBoundaries(text);
	const firstCharacterEnd = boundaries.find(boundary => boundary > firstNonWhitespaceIndex) ?? text.length;
	const firstCharacterWidth = measure(text.slice(firstNonWhitespaceIndex, firstCharacterEnd));
	const fullWidthCharacterWidth = measure('界');
	return indentWidth + Math.max(firstCharacterWidth, fullWidthCharacterWidth) > wrapWidth
		? 0
		: indentWidth;
}

function findFirstNonWhitespaceIndex(text: string): number {
	for (let index = 0; index < text.length; index += 1) {
		if (!/\s/u.test(text[index]!)) return index;
	}
	return -1;
}
