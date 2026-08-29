import { h } from '../../../../base/browser/dom.js';
import { BugIndicatingError } from '../../../../base/common/errors.js';
import { type IRasterizedGlyph } from '../raster/raster.js';
import { UsagePreviewColors, type ITextureAtlasAllocator, type ITextureAtlasPageGlyph } from './atlas.js';

export interface TextureAtlasSlabAllocatorOptions {
	readonly minimumSlabSize?: number;
}

export class TextureAtlasSlabAllocator implements ITextureAtlasAllocator {
	private readonly context: CanvasRenderingContext2D;
	private readonly allocatedGlyphs = new Set<Readonly<ITextureAtlasPageGlyph>>();
	private readonly minimumSlabSize: number;
	private currentX = 0;
	private currentY = 0;
	private rowHeight = 0;
	private nextIndex = 0;

	constructor(private readonly canvas: HTMLCanvasElement, private readonly textureIndex: number, options: TextureAtlasSlabAllocatorOptions = {}) {
		const context = canvas.getContext('2d', { alpha: true, willReadFrequently: true });
		if (!context) throw new Error('WebGPU texture atlas requires a 2D canvas context');
		this.context = context;
		this.minimumSlabSize = options.minimumSlabSize ?? 8;
	}

	public allocate(rasterizedGlyph: IRasterizedGlyph): ITextureAtlasPageGlyph | undefined {
		const width = rasterizedGlyph.boundingBox.right - rasterizedGlyph.boundingBox.left + 1;
		const height = rasterizedGlyph.boundingBox.bottom - rasterizedGlyph.boundingBox.top + 1;
		const slabSize = Math.max(this.minimumSlabSize, nextPowerOfTwo(Math.max(width, height)));
		if (slabSize > this.canvas.width || slabSize > this.canvas.height) throw new BugIndicatingError('Glyph is too large for the atlas page');
		if (this.currentX + slabSize > this.canvas.width) {
			this.currentX = 0;
			this.currentY += this.rowHeight;
			this.rowHeight = 0;
		}
		if (this.currentY + slabSize > this.canvas.height) return undefined;
		this.context.drawImage(rasterizedGlyph.source, rasterizedGlyph.boundingBox.left, rasterizedGlyph.boundingBox.top, width, height, this.currentX, this.currentY, width, height);
		const glyph = Object.freeze({
			pageIndex: this.textureIndex,
			glyphIndex: this.nextIndex++,
			x: this.currentX,
			y: this.currentY,
			w: width,
			h: height,
			originOffsetX: rasterizedGlyph.originOffset.x,
			originOffsetY: rasterizedGlyph.originOffset.y,
			advance: rasterizedGlyph.advance,
			fontBoundingBoxAscent: rasterizedGlyph.fontBoundingBoxAscent,
			fontBoundingBoxDescent: rasterizedGlyph.fontBoundingBoxDescent,
		});
		this.currentX += slabSize;
		this.rowHeight = Math.max(this.rowHeight, slabSize);
		this.allocatedGlyphs.add(glyph);
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
		for (const glyph of this.allocatedGlyphs) context.fillRect(glyph.x, glyph.y, glyph.w, glyph.h);
		return new Promise((resolve, reject) => canvas.toBlob(blob => blob ? resolve(blob) : reject(new Error('Could not create a texture atlas preview'))));
	}

	public getStats(): string {
		const usedPixels = [...this.allocatedGlyphs].reduce((total, glyph) => total + glyph.w * glyph.h, 0);
		const totalPixels = this.canvas.width * this.canvas.height;
		return `page${this.textureIndex}: ${usedPixels}/${totalPixels} pixels used`;
	}
}

function nextPowerOfTwo(value: number): number {
	return 2 ** Math.ceil(Math.log2(Math.max(1, value)));
}
