import { h } from '../../../../base/browser/dom.js';
import { BugIndicatingError } from '../../../../base/common/errors.js';
import { type IStyledRasterizedGlyph } from '../raster/raster.js';
import { UsagePreviewColors, type IStyledTextureAtlasAllocator, type IStyledTextureAtlasPageGlyph } from './atlas.js';

export interface TextureAtlasSlabAllocatorOptions {
	readonly minimumSlabSize?: number;
}

export class TextureAtlasSlabAllocator implements IStyledTextureAtlasAllocator {
	private readonly context: CanvasRenderingContext2D;
	private readonly glyphs = new Set<Readonly<IStyledTextureAtlasPageGlyph>>();
	private readonly minimumSlabSize: number;
	private x = 0;
	private y = 0;
	private rowHeight = 0;
	private nextIndex = 0;

	constructor(private readonly canvas: HTMLCanvasElement, private readonly pageIndex: number, options: TextureAtlasSlabAllocatorOptions = {}) {
		const context = canvas.getContext('2d', { alpha: true, willReadFrequently: true });
		if (!context) throw new Error('WebGPU texture atlas requires a 2D canvas context');
		this.context = context;
		this.minimumSlabSize = options.minimumSlabSize ?? 8;
	}

	public allocate(source: IStyledRasterizedGlyph): IStyledTextureAtlasPageGlyph | undefined {
		const width = source.boundingBox.right - source.boundingBox.left + 1;
		const height = source.boundingBox.bottom - source.boundingBox.top + 1;
		const slabSize = Math.max(this.minimumSlabSize, 2 ** Math.ceil(Math.log2(Math.max(1, width, height))));
		if (slabSize > this.canvas.width || slabSize > this.canvas.height) throw new BugIndicatingError('Glyph is too large for the atlas page');
		if (this.x + slabSize > this.canvas.width) {
			this.x = 0;
			this.y += this.rowHeight;
			this.rowHeight = 0;
		}
		if (this.y + slabSize > this.canvas.height) return undefined;
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
		this.x += slabSize;
		this.rowHeight = Math.max(this.rowHeight, slabSize);
		this.glyphs.add(glyph);
		return glyph;
	}

	public async getUsagePreview(): Promise<Blob> {
		const canvas = h(this.canvas.ownerDocument, 'canvas');
		canvas.width = this.canvas.width;
		canvas.height = this.canvas.height;
		const context = canvas.getContext('2d');
		if (!context) throw new Error('WebGPU texture atlas preview requires a 2D canvas context');
		context.fillStyle = UsagePreviewColors.Unused;
		context.fillRect(0, 0, canvas.width, canvas.height);
		context.fillStyle = UsagePreviewColors.Used;
		for (const glyph of this.glyphs) context.fillRect(glyph.x, glyph.y, glyph.w, glyph.h);
		return new Promise((resolve, reject) => canvas.toBlob(blob => blob ? resolve(blob) : reject(new Error('Could not create a texture atlas preview'))));
	}

	public getStats(): string {
		const used = [...this.glyphs].reduce((total, glyph) => total + glyph.w * glyph.h, 0);
		return `page${this.pageIndex}: ${used}/${this.canvas.width * this.canvas.height} pixels used`;
	}
}
