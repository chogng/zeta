import { h } from '../../../../base/browser/dom.js';
import { BugIndicatingError } from '../../../../base/common/errors.js';
import { type IStyledRasterizedGlyph } from '../raster/raster.js';
import { UsagePreviewColors, type IStyledTextureAtlasAllocator, type IStyledTextureAtlasPageGlyph } from './atlas.js';

export class TextureAtlasShelfAllocator implements IStyledTextureAtlasAllocator {
	private readonly context: CanvasRenderingContext2D;
	private readonly glyphs = new Set<Readonly<IStyledTextureAtlasPageGlyph>>();
	private x = 0;
	private y = 0;
	private rowHeight = 0;
	private nextIndex = 0;

	constructor(private readonly canvas: HTMLCanvasElement, private readonly pageIndex: number) {
		const context = canvas.getContext('2d', { alpha: true, willReadFrequently: true });
		if (!context) throw new Error('WebGPU texture atlas requires a 2D canvas context');
		this.context = context;
	}

	public allocate(source: IStyledRasterizedGlyph): IStyledTextureAtlasPageGlyph | undefined {
		const width = source.boundingBox.right - source.boundingBox.left + 1;
		const height = source.boundingBox.bottom - source.boundingBox.top + 1;
		if (width > this.canvas.width || height > this.canvas.height) throw new BugIndicatingError('Glyph is too large for the atlas page');
		if (this.x + width > this.canvas.width) {
			this.x = 0;
			this.y += this.rowHeight;
			this.rowHeight = 0;
		}
		if (this.y + height > this.canvas.height) return undefined;
		this.context.drawImage(source.source, source.boundingBox.left, source.boundingBox.top, width, height, this.x, this.y, width, height);
		const glyph = Object.freeze({
			pageIndex: this.pageIndex,
			glyphIndex: this.nextIndex++,
			x: this.x,
			y: this.y,
			w: width,
			h: height,
			originOffsetX: source.originOffset.x,
			originOffsetY: source.originOffset.y,
			advance: source.advance,
			fontBoundingBoxAscent: source.fontBoundingBoxAscent,
			fontBoundingBoxDescent: source.fontBoundingBoxDescent,
		});
		this.x += width;
		this.rowHeight = Math.max(this.rowHeight, height);
		this.glyphs.add(glyph);
		return glyph;
	}

	public getUsagePreview(): Promise<Blob> {
		return createPreview(this.canvas, this.glyphs);
	}

	public getStats(): string {
		const used = [...this.glyphs].reduce((total, glyph) => total + glyph.w * glyph.h, 0);
		return `page${this.pageIndex}: ${used}/${this.canvas.width * this.canvas.height} pixels used`;
	}
}

async function createPreview(source: HTMLCanvasElement, glyphs: ReadonlySet<Readonly<IStyledTextureAtlasPageGlyph>>): Promise<Blob> {
	const canvas = h(source.ownerDocument, 'canvas');
	canvas.width = source.width;
	canvas.height = source.height;
	const context = canvas.getContext('2d');
	if (!context) throw new Error('WebGPU texture atlas preview requires a 2D canvas context');
	context.fillStyle = UsagePreviewColors.Unused;
	context.fillRect(0, 0, canvas.width, canvas.height);
	context.fillStyle = UsagePreviewColors.Used;
	for (const glyph of glyphs) context.fillRect(glyph.x, glyph.y, glyph.w, glyph.h);
	return new Promise((resolve, reject) => canvas.toBlob(blob => blob ? resolve(blob) : reject(new Error('Could not create a texture atlas preview'))));
}
