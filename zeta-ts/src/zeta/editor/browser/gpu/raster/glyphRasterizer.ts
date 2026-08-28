import { type IGlyphRasterizer, type IGpuGlyphStyle, type IRasterizedGlyph } from './raster.js';
import { createCanvasFontShorthand } from '../../config/fontMeasurements.js';

let nextId = 0;

/** Rasterizes one grapheme using the browser canvas font stack selected by the editor. */
export class GlyphRasterizer implements IGlyphRasterizer {
	public readonly id = nextId++;
	public readonly cacheKey: string;
	private readonly canvas: HTMLCanvasElement;
	private readonly context: CanvasRenderingContext2D;

	constructor(ownerDocument: Document, public readonly devicePixelRatio: number) {
		this.cacheKey = String(devicePixelRatio);
		this.canvas = ownerDocument.createElement('canvas');
		const context = this.canvas.getContext('2d', { alpha: true, willReadFrequently: false });
		if (!context) throw new Error('WebGPU glyph rasterization requires a 2D canvas context');
		this.context = context;
	}

	public styleKey(style: IGpuGlyphStyle): string {
		return [style.color, style.fontFamily, style.fontSize, style.fontStyle, style.fontVariant, style.fontWeight, style.letterSpacing, this.devicePixelRatio].join('|');
	}

	public getTextMetrics(text: string, style: IGpuGlyphStyle): TextMetrics {
		this.applyRasterFont(style);
		return this.context.measureText(text);
	}

	public rasterizeGlyph(chars: string, style: IGpuGlyphStyle, subPixelX: number): IRasterizedGlyph {
		if (!chars) throw new TypeError('WebGPU glyph text must not be empty');
		const advance = this.getTextMetrics(chars, style).width + style.letterSpacing * this.devicePixelRatio;
		this.applyRasterFont(style);
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
		this.applyRasterFont(style);
		this.context.textBaseline = 'alphabetic';
		this.context.fillStyle = style.color;
		this.context.fillText(chars, padding + actualLeft + subPixelX, padding + actualAscent);
		const fontAscent = Math.max(actualAscent, metrics.fontBoundingBoxAscent || 0);
		const fontDescent = Math.max(actualDescent, metrics.fontBoundingBoxDescent || 0);
		return Object.freeze({
			source: this.canvas,
			boundingBox: Object.freeze({ left: 0, top: 0, right: width - 1, bottom: height - 1 }),
			originOffset: Object.freeze({ x: -actualLeft - padding, y: -actualAscent - padding }),
			advance,
			fontBoundingBoxAscent: fontAscent,
			fontBoundingBoxDescent: fontDescent,
		});
	}

	private applyRasterFont(style: IGpuGlyphStyle): void {
		this.applyFont(style, Math.ceil(style.fontSize * this.devicePixelRatio));
	}

	private applyFont(style: IGpuGlyphStyle, fontSize: number): void {
		this.context.font = createCanvasFontShorthand({
			style: style.fontStyle,
			variant: style.fontVariant,
			weight: style.fontWeight,
			size: `${fontSize}px`,
			family: style.fontFamily,
		});
	}
}
