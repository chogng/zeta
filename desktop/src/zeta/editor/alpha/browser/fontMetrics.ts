/**
 * Measures rendered Alpha line text for the current browser font.
 *
 * Implementations must be ready to measure immediately. `refresh` returns
 * whether cached line widths must be rebuilt after a font or style change.
 */
export interface AlphaTextMeasurer {
  readonly horizontalPadding: number;
  readonly contentLeftPadding: number;
  refresh(): boolean;
  measureLineWidth(text: string): number;
}

interface ResolvedFontMetrics {
  readonly signature: string;
  readonly canvasFont: string;
  readonly letterSpacing: number;
  readonly spaceWidth: number;
  readonly tabSize: number;
  readonly horizontalPadding: number;
  readonly contentLeftPadding: number;
  readonly fallbackCharacterWidth: number;
}

/**
 * Browser-backed line measurer using the Alpha line layer's computed style.
 *
 * Canvas handles shaping and font fallback. Tabs advance to CSS `tab-size`
 * stops based on the measured space glyph.
 */
export class AlphaDomTextMeasurer implements AlphaTextMeasurer {
  private readonly context: CanvasRenderingContext2D | undefined;
  private metrics: ResolvedFontMetrics;

  constructor(private readonly referenceElement: HTMLElement) {
    this.context = createCanvasContext(referenceElement.ownerDocument);
    this.metrics = this.readMetrics();
    this.configureContext();
  }

  get horizontalPadding(): number {
    return this.metrics.horizontalPadding;
  }

  get contentLeftPadding(): number {
    return this.metrics.contentLeftPadding;
  }

  refresh(): boolean {
    const next = this.readMetrics();
    if (next.signature === this.metrics.signature) return false;
    this.metrics = next;
    this.configureContext();
    return true;
  }

  measureLineWidth(text: string): number {
    if (!text.includes("\t")) return this.measureSegment(text);
    const tabStopWidth = this.metrics.spaceWidth * this.metrics.tabSize;
    let width = 0;
    const segments = text.split("\t");
    for (let index = 0; index < segments.length; index++) {
      width += this.measureSegment(segments[index] ?? "");
      if (index + 1 < segments.length) {
        width = (Math.floor(width / tabStopWidth) + 1) * tabStopWidth;
      }
    }
    return width;
  }

  private measureSegment(text: string): number {
    if (!text) return 0;
    const characterCount = [...text].length;
    const width = this.context
      ? this.context.measureText(text).width
      : characterCount * this.metrics.fallbackCharacterWidth;
    return Math.max(
      0,
      width + characterCount * this.metrics.letterSpacing,
    );
  }

  private readMetrics(): ResolvedFontMetrics {
    const ownerWindow = this.referenceElement.ownerDocument.defaultView;
    if (!ownerWindow) {
      throw new ReferenceError("Alpha font measurement requires a browser window");
    }
    const style = ownerWindow.getComputedStyle(this.referenceElement);
    const fontSize = positiveCssNumber(style.fontSize, 14);
    const letterSpacing = style.letterSpacing === "normal"
      ? 0
      : cssNumber(style.letterSpacing, 0);
    const tabSize = positiveCssNumber(style.tabSize, 4);
    const contentLeftPadding = cssNumber(style.paddingLeft, 0);
    const horizontalPadding =
      contentLeftPadding + cssNumber(style.paddingRight, 0);
    const canvasFont = [
      style.fontStyle || "normal",
      style.fontVariant || "normal",
      style.fontWeight || "400",
      style.fontStretch || "normal",
      style.fontSize || `${fontSize}px`,
      style.fontFamily || "monospace",
    ].join(" ");
    const fallbackCharacterWidth = fontSize * 0.6;
    const context = this.context;
    if (context) context.font = canvasFont;
    const spaceWidth = positiveNumber(
      context?.measureText(" ").width,
      fallbackCharacterWidth,
    );
    const signature = JSON.stringify([
      canvasFont,
      letterSpacing,
      style.fontFeatureSettings,
      style.fontKerning,
      style.fontVariationSettings,
      tabSize,
      horizontalPadding,
      contentLeftPadding,
      spaceWidth,
    ]);
    return {
      signature,
      canvasFont,
      letterSpacing,
      spaceWidth,
      tabSize,
      horizontalPadding,
      contentLeftPadding,
      fallbackCharacterWidth,
    };
  }

  private configureContext(): void {
    if (!this.context) return;
    this.context.font = this.metrics.canvasFont;
    this.context.textBaseline = "alphabetic";
  }
}

function createCanvasContext(
  ownerDocument: Document,
): CanvasRenderingContext2D | undefined {
  try {
    return ownerDocument.createElement("canvas").getContext("2d") ?? undefined;
  } catch {
    return undefined;
  }
}

function cssNumber(value: string, fallback: number): number {
  const parsed = Number.parseFloat(value);
  return Number.isFinite(parsed) ? parsed : fallback;
}

function positiveCssNumber(value: string, fallback: number): number {
  return positiveNumber(cssNumber(value, fallback), fallback);
}

function positiveNumber(
  value: number | undefined,
  fallback: number,
): number {
  return value !== undefined && Number.isFinite(value) && value > 0
    ? value
    : fallback;
}
