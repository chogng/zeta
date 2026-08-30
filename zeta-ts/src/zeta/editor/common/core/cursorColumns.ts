import { CharCode } from "../../../base/common/charCode.js";
import * as strings from "../../../base/common/strings.js";

/**
 * Converts UTF-16 columns to the editor's approximate visible columns.
 *
 * The calculation is intentionally font-independent. Browser measurement
 * remains the authority for rendered geometry; this utility is for cursor
 * navigation, indentation, and status-bar semantics.
 */
export class CursorColumns {
	private static _nextVisibleColumn(codePoint: number, visibleColumn: number, tabSize: number): number {
		if (codePoint === CharCode.Tab) return CursorColumns.nextRenderTabStop(visibleColumn, tabSize);
		return visibleColumn + (strings.isFullWidthCharacter(codePoint) || strings.isEmojiImprecise(codePoint) ? 2 : 1);
	}

	static visibleColumnFromColumn(lineContent: string, column: number, tabSize: number): number {
		const textLength = Math.min(column - 1, lineContent.length);
		const text = lineContent.substring(0, textLength);
		const iterator = new strings.GraphemeIterator(text);
		let visibleColumn = 0;
		while (!iterator.eol()) {
			const codePoint = strings.getNextCodePoint(text, textLength, iterator.offset);
			iterator.nextGraphemeLength();
			visibleColumn = this._nextVisibleColumn(codePoint, visibleColumn, tabSize);
		}
		return visibleColumn;
	}

	static columnFromVisibleColumn(lineContent: string, visibleColumn: number, tabSize: number): number {
		if (visibleColumn <= 0) return 1;
		const iterator = new strings.GraphemeIterator(lineContent);
		let beforeVisibleColumn = 0;
		let beforeColumn = 1;
		while (!iterator.eol()) {
			const codePoint = strings.getNextCodePoint(lineContent, lineContent.length, iterator.offset);
			iterator.nextGraphemeLength();
			const afterVisibleColumn = this._nextVisibleColumn(codePoint, beforeVisibleColumn, tabSize);
			const afterColumn = iterator.offset + 1;
			if (afterVisibleColumn >= visibleColumn) {
				return afterVisibleColumn - visibleColumn < visibleColumn - beforeVisibleColumn ? afterColumn : beforeColumn;
			}
			beforeVisibleColumn = afterVisibleColumn;
			beforeColumn = afterColumn;
		}
		return lineContent.length + 1;
	}

	static toStatusbarColumn(lineContent: string, columnIndex: number, tabSize: number): number {
		const text = lineContent.substring(0, Math.min(columnIndex - 1, lineContent.length));
		const iterator = new strings.CodePointIterator(text);
		let result = 0;
		while (!iterator.eol()) {
			result = iterator.nextCodePoint() === CharCode.Tab ? CursorColumns.nextRenderTabStop(result, tabSize) : result + 1;
		}
		return result + 1;
	}

	static nextRenderTabStop(visibleColumn: number, tabSize: number): number {
		return visibleColumn + tabSize - visibleColumn % tabSize;
	}

	static nextIndentTabStop(visibleColumn: number, indentSize: number): number {
		return this.nextRenderTabStop(visibleColumn, indentSize);
	}

	static prevRenderTabStop(visibleColumn: number, tabSize: number): number {
		return Math.max(0, visibleColumn - 1 - (visibleColumn - 1) % tabSize);
	}

	static prevIndentTabStop(visibleColumn: number, indentSize: number): number {
		return this.prevRenderTabStop(visibleColumn, indentSize);
	}
}
