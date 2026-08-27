import { type GpuGlyphStyle, GlyphRasterizer } from '../raster/glyphRasterizer.js';

export interface TextureAtlasGlyph {
	readonly pageIndex: number;
	readonly x: number;
	readonly y: number;
	readonly width: number;
	readonly height: number;
	readonly offsetX: number;
	readonly offsetY: number;
	readonly advance: number;
	readonly fontAscent: number;
	readonly fontDescent: number;
}

export interface ReadonlyTextureAtlasPage {
	readonly index: number;
	readonly source: HTMLCanvasElement;
	readonly version: number;
	readonly usedWidth: number;
	readonly usedHeight: number;
}

/** Browser-rasterized glyph pages uploaded by the WebGPU text renderer. */
export class TextureAtlas {
	public static readonly maximumPageCount = 16;
	private readonly glyphs = new Map<string, TextureAtlasGlyph>();
	private readonly mutablePages: TextureAtlasPage[] = [];

	constructor(private readonly ownerDocument: Document, public readonly pageSize: number) {
		if (!Number.isSafeInteger(pageSize) || pageSize < 64) throw new RangeError('WebGPU texture atlas page size must be an integer of at least 64 pixels');
		this.mutablePages.push(new TextureAtlasPage(ownerDocument, 0, pageSize));
	}

	public get pages(): readonly ReadonlyTextureAtlasPage[] {
		return this.mutablePages;
	}

	public getGlyph(rasterizer: GlyphRasterizer, chars: string, style: GpuGlyphStyle, deviceX: number): TextureAtlasGlyph {
		const subPixelBucket = Math.round((deviceX - Math.floor(deviceX)) * 10) % 10;
		const key = `${rasterizer.styleKey(style)}|${subPixelBucket}|${chars}`;
		const existing = this.glyphs.get(key);
		if (existing) return existing;
		const rasterized = rasterizer.rasterizeGlyph(chars, style, subPixelBucket / 10);
		for (const page of this.mutablePages) {
			const glyph = page.allocate(rasterized);
			if (glyph) {
				this.glyphs.set(key, glyph);
				return glyph;
			}
		}
		if (this.mutablePages.length >= TextureAtlas.maximumPageCount) throw new RangeError('WebGPU texture atlas exhausted its page limit');
		const page = new TextureAtlasPage(this.ownerDocument, this.mutablePages.length, this.pageSize);
		this.mutablePages.push(page);
		const glyph = page.allocate(rasterized);
		if (!glyph) throw new RangeError('WebGPU glyph exceeds the texture atlas page size');
		this.glyphs.set(key, glyph);
		return glyph;
	}

	public clear(): void {
		this.glyphs.clear();
		this.mutablePages.splice(0, this.mutablePages.length, new TextureAtlasPage(this.ownerDocument, 0, this.pageSize));
	}
}

type RasterizedGlyph = ReturnType<GlyphRasterizer['rasterizeGlyph']>;

class TextureAtlasPage implements ReadonlyTextureAtlasPage {
	public readonly source: HTMLCanvasElement;
	private readonly context: CanvasRenderingContext2D;
	private nextX = 1;
	private nextY = 1;
	private rowHeight = 0;
	public version = 0;
	public usedWidth = 1;
	public usedHeight = 1;

	constructor(ownerDocument: Document, public readonly index: number, private readonly size: number) {
		this.source = ownerDocument.createElement('canvas');
		this.source.width = size;
		this.source.height = size;
		const context = this.source.getContext('2d', { alpha: true });
		if (!context) throw new Error('WebGPU texture atlas requires a 2D canvas context');
		this.context = context;
	}

	public allocate(rasterized: RasterizedGlyph): TextureAtlasGlyph | undefined {
		const allocationWidth = rasterized.width + 2;
		const allocationHeight = rasterized.height + 2;
		if (allocationWidth > this.size || allocationHeight > this.size) return undefined;
		if (this.nextX + allocationWidth > this.size) {
			this.nextX = 1;
			this.nextY += this.rowHeight;
			this.rowHeight = 0;
		}
		if (this.nextY + allocationHeight > this.size) return undefined;
		const x = this.nextX + 1;
		const y = this.nextY + 1;
		this.context.drawImage(rasterized.source, 0, 0, rasterized.width, rasterized.height, x, y, rasterized.width, rasterized.height);
		this.nextX += allocationWidth;
		this.rowHeight = Math.max(this.rowHeight, allocationHeight);
		this.usedWidth = Math.max(this.usedWidth, x + rasterized.width);
		this.usedHeight = Math.max(this.usedHeight, y + rasterized.height);
		this.version += 1;
		return Object.freeze({
			pageIndex: this.index,
			x,
			y,
			width: rasterized.width,
			height: rasterized.height,
			offsetX: rasterized.offsetX,
			offsetY: rasterized.offsetY,
			advance: rasterized.advance,
			fontAscent: rasterized.fontAscent,
			fontDescent: rasterized.fontDescent,
		});
	}
}
