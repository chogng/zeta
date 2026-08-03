/** Minimal document access required to derive Alpha's bounded minimap density rows. */
export interface AlphaMinimapTextSource {
  readonly lineCount: number;
  getLineContent(lineIndex: number): string;
}

export interface AlphaMinimapRow {
  readonly startLineIndex: number;
  readonly endLineIndexExclusive: number;
  /** Relative non-whitespace content density, normalized to the sampled document maximum. */
  readonly density: number;
}

export const ALPHA_MINIMAP_MAX_ROWS = 160;

/**
 * Projects a document into a bounded density strip without retaining its text.
 *
 * Each row samples a fixed number of lines from its source interval, keeping
 * redraw work bounded even for very large files. The minimap is a navigation
 * preview, not a syntax or exact-glyph layout surface.
 */
export function createAlphaMinimapRows(source: AlphaMinimapTextSource, maximumRows = ALPHA_MINIMAP_MAX_ROWS): readonly AlphaMinimapRow[] {
  if (!source || !Number.isSafeInteger(source.lineCount) || source.lineCount < 1 || typeof source.getLineContent !== "function") {
    throw new TypeError("Alpha minimap requires a non-empty text source");
  }
  if (!Number.isSafeInteger(maximumRows) || maximumRows < 1) {
    throw new RangeError("Alpha minimap maximum rows must be a positive safe integer");
  }
  const rowCount = Math.min(source.lineCount, maximumRows);
  const sampled = Array.from({ length: rowCount }, (_, rowIndex) => {
    const startLineIndex = Math.floor(rowIndex * source.lineCount / rowCount);
    const endLineIndexExclusive = Math.floor((rowIndex + 1) * source.lineCount / rowCount);
    return Object.freeze({
      startLineIndex,
      endLineIndexExclusive,
      contentLength: sampledContentLength(source, startLineIndex, endLineIndexExclusive),
    });
  });
  const maximumContentLength = Math.max(...sampled.map(row => row.contentLength));
  if (maximumContentLength === 0) return Object.freeze([]);
  return Object.freeze(sampled.flatMap(row => row.contentLength === 0 ? [] : [Object.freeze({
    startLineIndex: row.startLineIndex,
    endLineIndexExclusive: row.endLineIndexExclusive,
    density: row.contentLength / maximumContentLength,
  })]));
}

function sampledContentLength(source: AlphaMinimapTextSource, startLineIndex: number, endLineIndexExclusive: number): number {
  const lineCount = endLineIndexExclusive - startLineIndex;
  const samples = Math.min(4, lineCount);
  let maximum = 0;
  for (let sampleIndex = 0; sampleIndex < samples; sampleIndex += 1) {
    const lineIndex = startLineIndex + Math.floor(sampleIndex * lineCount / samples);
    const text = source.getLineContent(lineIndex);
    if (typeof text !== "string") throw new TypeError("Alpha minimap text source returned non-text content");
    maximum = Math.max(maximum, [...text.trimEnd()].length);
  }
  return maximum;
}
