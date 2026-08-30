import { h } from '../../../../base/browser/dom.js';
import { type IStyledGlyphRasterizer, type IStyledGlyphStyle, type IStyledRasterizedGlyph } from './raster.js';

let nextId = 0;

/** Rasterizes a grapheme with the resolved font and color of the editor row. */
export class GlyphRasterizer implements IStyledGlyphRasterizer {
	public readonly id = nextId++;
	public readonly cacheKey: string;
	private readonly canvas: HTMLCanvasElement;
	private readonly context: CanvasRenderingContext2D;

	constructor(host: HTMLElement, public readonly devicePixelRatio: number) {
		this.cacheKey = String(devicePixelRatio);
		this.canvas = h(host.ownerDocument, 'canvas');
		const context = this.canvas.getContext('2d', { alpha: true });
		if (!context) throw new Error('WebGPU glyph rasterization requires a 2D canvas context');
		this.context = context;
	}

	public styleKey(style: IStyledGlyphStyle): string {
		return JSON.stringify([style.color, style.fontFamily, style.fontSize, style.fontStyle, style.fontVariant, style.fontWeight, style.letterSpacing, this.devicePixelRatio]);
	}

	public getTextMetrics(text: string, style: IStyledGlyphStyle): TextMetrics {
		this.applyFont(style, style.fontSize * this.devicePixelRatio);
		return this.context.measureText(text);
	}

	public rasterizeGlyph(chars: string, style: IStyledGlyphStyle, subPixelX: number): IStyledRasterizedGlyph {
		if (!chars) throw new TypeError('WebGPU glyph text must not be empty');
		const advance = this.getTextMetrics(chars, style).width + style.letterSpacing * this.devicePixelRatio;
		this.applyFont(style, Math.ceil(style.fontSize * this.devicePixelRatio));
		const metrics = this.context.measureText(chars);
		const ascent = Math.max(0, metrics.actualBoundingBoxAscent);
		const descent = Math.max(0, metrics.actualBoundingBoxDescent);
		const left = Math.max(0, metrics.actualBoundingBoxLeft);
		const right = Math.max(0, metrics.actualBoundingBoxRight);
		const padding = 2;
		const width = Math.max(1, Math.ceil(left + right + padding * 2 + 1));
		const height = Math.max(1, Math.ceil(ascent + descent + padding * 2 + 1));
		if (this.canvas.width !== width || this.canvas.height !== height) {
			this.canvas.width = width;
			this.canvas.height = height;
		}
		this.context.clearRect(0, 0, width, height);
		this.applyFont(style, Math.ceil(style.fontSize * this.devicePixelRatio));
		this.context.textBaseline = 'alphabetic';
		this.context.fillStyle = style.color;
		this.context.fillText(chars, padding + left + subPixelX, padding + ascent);
		return Object.freeze({
			source: this.canvas,
			boundingBox: Object.freeze({ left: 0, top: 0, right: width - 1, bottom: height - 1 }),
			originOffset: Object.freeze({ x: -left - padding, y: -ascent - padding }),
			advance,
			fontBoundingBoxAscent: Math.max(ascent, metrics.fontBoundingBoxAscent || 0),
			fontBoundingBoxDescent: Math.max(descent, metrics.fontBoundingBoxDescent || 0),
		});
	}

	private applyFont(style: IStyledGlyphStyle, fontSize: number): void {
		this.context.font = `${style.fontStyle} ${style.fontVariant} ${style.fontWeight} ${fontSize}px ${style.fontFamily}`;
	}
}
