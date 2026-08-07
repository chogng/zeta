import { TextPosition, TextRange } from "../../../common/core/text.js";
import { type TextModel } from "../../../common/model/textModel.js";

export type UnicodeHighlightKind = "invisible" | "bidi" | "confusable";

export interface UnicodeHighlight { readonly range: TextRange; readonly kind: UnicodeHighlightKind; readonly character: string; }

/** Finds editor-dangerous invisible, bidi-control, and likely confusable characters. */
export function findUnicodeHighlights(model: TextModel): readonly UnicodeHighlight[] {
  const result: UnicodeHighlight[] = [];
  for (let lineIndex = 0; lineIndex < model.lineCount; lineIndex += 1) {
    const line = model.getLineContent(lineIndex);
    let columnIndex = 0;
    for (const character of line) {
      const end = columnIndex + character.length;
      const kind = classifyCharacter(character, line);
      if (kind) result.push(Object.freeze({ range: TextRange.from(TextPosition.at(lineIndex, columnIndex), TextPosition.at(lineIndex, end)), kind, character }));
      columnIndex = end;
    }
  }
  return Object.freeze(result);
}

function classifyCharacter(character: string, line: string): UnicodeHighlightKind | undefined {
  const codePoint = character.codePointAt(0)!;
  if (isBidiControl(codePoint)) return "bidi";
  if (isInvisible(codePoint)) return "invisible";
  if (isConfusable(character, line)) return "confusable";
  return undefined;
}

function isBidiControl(codePoint: number): boolean { return (codePoint >= 0x202a && codePoint <= 0x202e) || (codePoint >= 0x2066 && codePoint <= 0x2069); }
function isInvisible(codePoint: number): boolean { return codePoint === 0x00ad || codePoint === 0x061c || codePoint === 0x200b || codePoint === 0x200c || codePoint === 0x200d || codePoint === 0x2060 || codePoint === 0xfeff || (codePoint >= 0 && codePoint < 0x20 && codePoint !== 0x09); }
function isConfusable(character: string, line: string): boolean {
  if (!/[\u0370-\u03ff\u0400-\u04ff]/u.test(character)) return false;
  return /[A-Za-z]/u.test(line) && /[A-Za-z0-9_$]/u.test(line);
}
