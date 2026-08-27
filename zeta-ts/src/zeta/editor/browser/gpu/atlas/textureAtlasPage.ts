import { Disposable, toDisposable } from '../../../../base/common/lifecycle.js';
import { type IGlyphRasterizer, type IRasterizedGlyph } from '../raster/raster.js';
import { type IReadableTextureAtlasPage, type ITextureAtlasAllocator, type ITextureAtlasPageGlyph } from './atlas.js';
import { TextureAtlasShelfAllocator } from './textureAtlasShelfAllocator.js';
import { TextureAtlasSlabAllocator } from './textureAtlasSlabAllocator.js';

export type AllocatorType = 'shelf' | 'slab' | ((canvas: HTMLCanvasElement, textureIndex: number) => ITextureAtlasAllocator);

export class TextureAtlasPage extends Disposable implements IReadableTextureAtlasPage {
	public static readonly maximumGlyphCount = 2_500;
	public readonly source: HTMLCanvasElement;
	public readonly glyphs = new Set<Readonly<ITextureAtlasPageGlyph>>();
	private readonly allocator: ITextureAtlasAllocator;
	private readonly glyphMap = new Map<string, Readonly<ITextureAtlasPageGlyph>>();
	private currentVersion = 0;
	private mutableUsedArea = { left: 0, top: 0, right: 0, bottom: 0 };

	constructor(ownerDocument: Document, public readonly index: number, pageSize: number, allocatorType: AllocatorType = 'slab') {
		super();
		this.source = ownerDocument.createElement('canvas');
		this.source.width = pageSize;
		this.source.height = pageSize;
		this.allocator = typeof allocatorType === 'function'
			? allocatorType(this.source, index)
			: allocatorType === 'shelf'
				? new TextureAtlasShelfAllocator(this.source, index)
				: new TextureAtlasSlabAllocator(this.source, index);
		this._register(toDisposable(() => {
			this.source.width = 1;
			this.source.height = 1;
		}));
	}

	public get version(): number {
		return this.currentVersion;
	}

	public get usedArea(): Readonly<{ readonly left: number; readonly top: number; readonly right: number; readonly bottom: number }> {
		return this.mutableUsedArea;
	}

	public getGlyph(rasterizer: IGlyphRasterizer, chars: string, styleKey: string, rasterize: () => IRasterizedGlyph): Readonly<ITextureAtlasPageGlyph> | undefined {
		const key = `${rasterizer.cacheKey}|${styleKey}|${chars}`;
		const existing = this.glyphMap.get(key);
		if (existing) return existing;
		if (this.glyphs.size >= TextureAtlasPage.maximumGlyphCount) return undefined;
		const glyph = this.allocator.allocate(rasterize());
		if (!glyph) return undefined;
		this.glyphMap.set(key, glyph);
		this.glyphs.add(glyph);
		this.currentVersion += 1;
		this.mutableUsedArea = Object.freeze({
			left: 0,
			top: 0,
			right: Math.max(this.mutableUsedArea.right, glyph.x + glyph.w),
			bottom: Math.max(this.mutableUsedArea.bottom, glyph.y + glyph.h),
		});
		return glyph;
	}

	public getUsagePreview(): Promise<Blob> {
		return this.allocator.getUsagePreview();
	}

	public getStats(): string {
		return this.allocator.getStats();
	}
}
