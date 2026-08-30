import { h } from '../../../../base/browser/dom.js';
import { BugIndicatingError } from '../../../../base/common/errors.js';
import { type IStyledRasterizedGlyph } from '../raster/raster.js';
import { UsagePreviewColors, type IStyledTextureAtlasAllocator, type IStyledTextureAtlasPageGlyph } from './atlas.js';

export class StyledTextureAtlasShelfAllocator implements IStyledTextureAtlasAllocator {
	private readonly context: CanvasRenderingContext2D;
	private readonly allocatedGlyphs = new Set<Readonly<IStyledTextureAtlasPageGlyph>>();
	private currentX = 0;
	private currentY = 0;
	private rowHeight = 0;
	private nextIndex = 0;

	constructor(private readonly canvas: HTMLCanvasElement, private readonly textureIndex: number) {
		const context = canvas.getContext('2d', { alpha: true, willReadFrequently: true });
		if (!context) throw new Error('WebGPU texture atlas requires a 2D canvas context');
		this.context = context;
	}

	public allocate(rasterizedGlyph: IStyledRasterizedGlyph): IStyledTextureAtlasPageGlyph | undefined {
		const width = rasterizedGlyph.boundingBox.right - rasterizedGlyph.boundingBox.left + 1;
		const height = rasterizedGlyph.boundingBox.bottom - rasterizedGlyph.boundingBox.top + 1;
		if (width > this.canvas.width || height > this.canvas.height) throw new BugIndicatingError('Glyph is too large for the atlas page');
		if (this.currentX + width > this.canvas.width) {
			this.currentX = 0;
			this.currentY += this.rowHeight;
			this.rowHeight = 0;
		}
		if (this.currentY + height > this.canvas.height) return undefined;
		this.context.drawImage(rasterizedGlyph.source, rasterizedGlyph.boundingBox.left, rasterizedGlyph.boundingBox.top, width, height, this.currentX, this.currentY, width, height);
		const glyph = this.createGlyph(rasterizedGlyph, this.currentX, this.currentY, width, height);
		this.currentX += width;
		this.rowHeight = Math.max(this.rowHeight, height);
		this.allocatedGlyphs.add(glyph);
		return glyph;
	}

	public async getUsagePreview(): Promise<Blob> {
		return canvasToBlob(this.canvas, this.allocatedGlyphs);
	}

	public getStats(): string {
		const usedPixels = [...this.allocatedGlyphs].reduce((total, glyph) => total + glyph.w * glyph.h, 0);
		const totalPixels = this.canvas.width * this.canvas.height;
		return `page${this.textureIndex}: ${usedPixels}/${totalPixels} pixels used`;
	}

	private createGlyph(rasterizedGlyph: IStyledRasterizedGlyph, x: number, y: number, width: number, height: number): IStyledTextureAtlasPageGlyph {
		return Object.freeze({
			pageIndex: this.textureIndex,
			glyphIndex: this.nextIndex++,
			x,
			y,
			w: width,
			h: height,
			originOffsetX: rasterizedGlyph.originOffset.x,
			originOffsetY: rasterizedGlyph.originOffset.y,
			advance: rasterizedGlyph.advance,
			fontBoundingBoxAscent: rasterizedGlyph.fontBoundingBoxAscent,
			fontBoundingBoxDescent: rasterizedGlyph.fontBoundingBoxDescent,
		});
	}
}

async function canvasToBlob(source: HTMLCanvasElement, glyphs: ReadonlySet<Readonly<IStyledTextureAtlasPageGlyph>>): Promise<Blob> {
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
