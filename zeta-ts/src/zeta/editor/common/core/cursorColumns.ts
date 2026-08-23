/**
 * Converts UTF-16 columns to the editor's approximate visible columns.
 *
 * The calculation is intentionally font-independent. Browser measurement
 * remains the authority for rendered geometry; this utility is for cursor
 * navigation, indentation, and status-bar semantics.
 */
export class CursorColumns {
  static visibleColumnFromColumn(lineContent: string, columnIndex: number, tabSize: number): number {
    validateTabSize(tabSize);
    const end = Math.min(Math.max(0, columnIndex), lineContent.length);
    let visibleColumn = 0;
    for (let index = 0; index < end;) {
      const codePoint = lineContent.codePointAt(index)!;
      const width = codePoint > 0xffff ? 2 : 1;
      visibleColumn = nextVisibleColumn(codePoint, visibleColumn, tabSize);
      index += width;
    }
    return visibleColumn;
  }

  static columnFromVisibleColumn(lineContent: string, visibleColumn: number, tabSize: number): number {
    validateTabSize(tabSize);
    if (visibleColumn <= 0) return 0;
    let currentColumn = 0;
    let currentVisibleColumn = 0;
    while (currentColumn < lineContent.length) {
      const codePoint = lineContent.codePointAt(currentColumn)!;
      const width = codePoint > 0xffff ? 2 : 1;
      const next = nextVisibleColumn(codePoint, currentVisibleColumn, tabSize);
      if (next >= visibleColumn) {
        return next - visibleColumn < visibleColumn - currentVisibleColumn
          ? currentColumn + width
          : currentColumn;
      }
      currentColumn += width;
      currentVisibleColumn = next;
    }
    return lineContent.length;
  }

  static toStatusbarColumn(lineContent: string, columnIndex: number, tabSize: number): number {
    return this.visibleColumnFromColumn(lineContent, columnIndex, tabSize) + 1;
  }

  static nextRenderTabStop(visibleColumn: number, tabSize: number): number {
    validateTabSize(tabSize);
    return visibleColumn + tabSize - visibleColumn % tabSize;
  }

  static nextIndentTabStop(visibleColumn: number, indentSize: number): number {
    return this.nextRenderTabStop(visibleColumn, indentSize);
  }

  static prevRenderTabStop(visibleColumn: number, tabSize: number): number {
    validateTabSize(tabSize);
    return Math.max(0, visibleColumn - 1 - (visibleColumn - 1) % tabSize);
  }

  static prevIndentTabStop(visibleColumn: number, indentSize: number): number {
    return this.prevRenderTabStop(visibleColumn, indentSize);
  }
}

function nextVisibleColumn(codePoint: number, visibleColumn: number, tabSize: number): number {
  if (codePoint === 9) return CursorColumns.nextRenderTabStop(visibleColumn, tabSize);
  return visibleColumn + (isWideCodePoint(codePoint) ? 2 : 1);
}

function isWideCodePoint(codePoint: number): boolean {
  return codePoint >= 0x1100 && (
    codePoint <= 0x115f ||
    codePoint === 0x2329 ||
    codePoint === 0x232a ||
    (codePoint >= 0x2e80 && codePoint <= 0xa4cf) ||
    (codePoint >= 0xac00 && codePoint <= 0xd7a3) ||
    (codePoint >= 0xf900 && codePoint <= 0xfaff) ||
    (codePoint >= 0xfe10 && codePoint <= 0xfe19) ||
    (codePoint >= 0xfe30 && codePoint <= 0xfe6f) ||
    (codePoint >= 0xff00 && codePoint <= 0xff60) ||
    (codePoint >= 0xffe0 && codePoint <= 0xffe6)
  );
}

function validateTabSize(value: number): void {
  if (!Number.isSafeInteger(value) || value <= 0) throw new RangeError("Tab size must be a positive safe integer");
}
