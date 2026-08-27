export interface GpuGlyphStyle {
	readonly color: string;
	readonly fontFamily: string;
	readonly fontSize: number;
	readonly fontStyle: string;
	readonly fontVariant: string;
	readonly fontWeight: string;
	readonly letterSpacing: number;
}

export interface RasterizedGlyph {
	readonly source: HTMLCanvasElement;
	readonly width: number;
	readonly height: number;
	readonly offsetX: number;
	readonly offsetY: number;
	readonly advance: number;
	readonly fontAscent: number;
	readonly fontDescent: number;
}

let nextId = 0;

/** Rasterizes one grapheme using the browser canvas font stack selected by the editor. */
export class GlyphRasterizer {
	public readonly id = nextId++;
	private readonly canvas: HTMLCanvasElement;
	private readonly context: CanvasRenderingContext2D;

	constructor(ownerDocument: Document, public readonly devicePixelRatio: number) {
		this.canvas = ownerDocument.createElement('canvas');
		const context = this.canvas.getContext('2d', { alpha: true, willReadFrequently: false });
		if (!context) throw new Error('WebGPU glyph rasterization requires a 2D canvas context');
		this.context = context;
	}

	public styleKey(style: GpuGlyphStyle): string {
		return [style.color, style.fontFamily, style.fontSize, style.fontStyle, style.fontVariant, style.fontWeight, style.letterSpacing, this.devicePixelRatio].join('|');
	}

	public rasterizeGlyph(chars: string, style: GpuGlyphStyle, subPixelX: number): RasterizedGlyph {
		if (!chars) throw new TypeError('WebGPU glyph text must not be empty');
		const fontSize = Math.ceil(style.fontSize * this.devicePixelRatio);
		const font = `${style.fontStyle} ${style.fontVariant} ${style.fontWeight} ${fontSize}px ${style.fontFamily}`;
		this.context.font = font;
		const metrics = this.context.measureText(chars);
		const actualAscent = Math.max(0, metrics.actualBoundingBoxAscent);
		const actualDescent = Math.max(0, metrics.actualBoundingBoxDescent);
		const actualLeft = Math.max(0, metrics.actualBoundingBoxLeft);
		const actualRight = Math.max(0, metrics.actualBoundingBoxRight);
		const padding = 2;
		const width = Math.max(1, Math.ceil(actualLeft + actualRight + padding * 2 + 1));
		const height = Math.max(1, Math.ceil(actualAscent + actualDescent + padding * 2 + 1));
		if (this.canvas.width !== width || this.canvas.height !== height) {
			this.canvas.width = width;
			this.canvas.height = height;
		}
		this.context.clearRect(0, 0, width, height);
		this.context.font = font;
		this.context.textBaseline = 'alphabetic';
		this.context.fillStyle = style.color;
		this.context.fillText(chars, padding + actualLeft + subPixelX, padding + actualAscent);
		const fontAscent = Math.max(actualAscent, metrics.fontBoundingBoxAscent || 0);
		const fontDescent = Math.max(actualDescent, metrics.fontBoundingBoxDescent || 0);
		return Object.freeze({
			source: this.canvas,
			width,
			height,
			offsetX: -actualLeft - padding,
			offsetY: -actualAscent - padding,
			advance: metrics.width + style.letterSpacing * this.devicePixelRatio,
			fontAscent,
			fontDescent,
		});
	}
}
