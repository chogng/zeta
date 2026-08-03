import { EditorClipboardPasteMode, type EditorClipboardEntry } from "../common/clipboard.js";
import { type TextPosition } from "../common/text.js";
import { AlphaSemanticTokenPresentation, type AlphaSemanticTokenSource } from "./semanticTokenPresentation.js";

/**
 * Creates portable preformatted HTML for an Alpha clipboard operation.
 *
 * Plain text remains authoritative for paste. Syntax markup is a best-effort
 * representation of the current, version-bound browser token source and never
 * changes the copied characters.
 */
export function createAlphaSyntaxClipboardHtml(entries: readonly EditorClipboardEntry[], lineEnding: "\n" | "\r\n", tokens: AlphaSemanticTokenSource | undefined, ownerDocument: Document): string {
  const included = entries.filter(entry => entry.text.length > 0);
  const contents = included.map(entry => renderClipboardEntry(entry, tokens, ownerDocument));
  const separators = included.map((entry, index) => index === 0 || included[index - 1]!.pasteMode === EditorClipboardPasteMode.Line ? "" : "\n");
  return `<pre><code>${toExternalLineEndings(contents.map((content, index) => `${separators[index]}${content}`).join(""), lineEnding)}</code></pre>`;
}

function renderClipboardEntry(entry: EditorClipboardEntry, tokens: AlphaSemanticTokenSource | undefined, ownerDocument: Document): string {
  if (!tokens) return escapeHtml(entry.text);
  try {
    const model = tokens.textModel;
    const endOffset = model.offsetAt(entry.sourceRange.end);
    const exactStartOffset = endOffset - entry.text.length;
    if (exactStartOffset >= 0 && model.getText().slice(exactStartOffset, endOffset) === entry.text) {
      return renderTokenizedRange(tokens, model.positionAt(exactStartOffset), entry.sourceRange.end, ownerDocument);
    }
    if (entry.pasteMode !== EditorClipboardPasteMode.Line || !entry.text.endsWith("\n")) {
      return escapeHtml(entry.text);
    }
    const lineText = entry.text.slice(0, -1);
    const contentEndOffset = model.getText()[endOffset - 1] === "\n"
      ? endOffset - 1
      : endOffset;
    const contentStartOffset = contentEndOffset - lineText.length;
    if (contentStartOffset < 0 || model.getText().slice(contentStartOffset, contentEndOffset) !== lineText) {
      return escapeHtml(entry.text);
    }
    return `${renderTokenizedRange(tokens, model.positionAt(contentStartOffset), model.positionAt(contentEndOffset), ownerDocument)}\n`;
  } catch {
    return escapeHtml(entry.text);
  }
}

function renderTokenizedRange(tokens: AlphaSemanticTokenSource, start: TextPosition, end: TextPosition, ownerDocument: Document): string {
  const parts: string[] = [];
  const model = tokens.textModel;
  const colors = resolveTokenColors(ownerDocument);
  for (let lineIndex = start.lineIndex; lineIndex <= end.lineIndex; lineIndex += 1) {
    const lineText = model.getLineContent(lineIndex);
    const startColumn = lineIndex === start.lineIndex ? start.columnIndex : 0;
    const endColumn = lineIndex === end.lineIndex ? end.columnIndex : lineText.length;
    parts.push(renderTokenizedLine(
      lineText,
      startColumn,
      endColumn,
      tokens.getLineTokens(lineIndex),
      colors,
    ));
    if (lineIndex < end.lineIndex) parts.push("\n");
  }
  return parts.join("");
}

function renderTokenizedLine(lineText: string, startColumn: number, endColumn: number, tokens: ReturnType<AlphaSemanticTokenSource["getLineTokens"]>, colors: ReadonlyMap<AlphaSemanticTokenPresentation, string>): string {
  let column = startColumn;
  const parts: string[] = [];
  for (const token of tokens) {
    const tokenStart = Math.max(startColumn, token.startColumn);
    const tokenEnd = Math.min(endColumn, token.endColumn);
    if (tokenEnd <= tokenStart) continue;
    if (column < tokenStart) parts.push(escapeHtml(lineText.slice(column, tokenStart)));
    const color = colors.get(token.presentation);
    const style = color ? ` style="color: ${escapeHtml(color)}"` : "";
    const modifiers = token.modifiers?.join(" ") ?? "";
    parts.push(`<span class="zeta-alpha-editor-token ${token.presentation}${modifiers ? ` ${modifiers}` : ""}"${style}>${escapeHtml(lineText.slice(tokenStart, tokenEnd))}</span>`);
    column = tokenEnd;
  }
  if (column < endColumn) parts.push(escapeHtml(lineText.slice(column, endColumn)));
  return parts.join("");
}

function resolveTokenColors(ownerDocument: Document): ReadonlyMap<AlphaSemanticTokenPresentation, string> {
  const view = ownerDocument.defaultView;
  if (!view) return new Map();
  const style = view.getComputedStyle(ownerDocument.documentElement);
  const colors = new Map<AlphaSemanticTokenPresentation, string>();
  for (const [presentation, variable] of TOKEN_COLOR_VARIABLES) {
    const color = style.getPropertyValue(variable).trim();
    if (color.length > 0) colors.set(presentation, color);
  }
  return colors;
}

const TOKEN_COLOR_VARIABLES = new Map<AlphaSemanticTokenPresentation, string>([
  [AlphaSemanticTokenPresentation.Comment, "--zeta-editor-token-comment-foreground"],
  [AlphaSemanticTokenPresentation.Keyword, "--zeta-editor-token-keyword-foreground"],
  [AlphaSemanticTokenPresentation.String, "--zeta-editor-token-string-foreground"],
  [AlphaSemanticTokenPresentation.Number, "--zeta-editor-token-number-foreground"],
  [AlphaSemanticTokenPresentation.Regexp, "--zeta-editor-token-regexp-foreground"],
  [AlphaSemanticTokenPresentation.Type, "--zeta-editor-token-type-foreground"],
  [AlphaSemanticTokenPresentation.Function, "--zeta-editor-token-function-foreground"],
  [AlphaSemanticTokenPresentation.Variable, "--zeta-editor-token-variable-foreground"],
  [AlphaSemanticTokenPresentation.Operator, "--zeta-editor-token-operator-foreground"],
]);

function toExternalLineEndings(text: string, lineEnding: "\n" | "\r\n"): string {
  return lineEnding === "\n" ? text : text.replaceAll("\n", "\r\n");
}

function escapeHtml(text: string): string {
  return text.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;").replaceAll("\"", "&quot;").replaceAll("'", "&#39;");
}
