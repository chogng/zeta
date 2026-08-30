import { h } from '../../../../base/browser/dom.js';
import { Disposable, toDisposable } from '../../../../base/common/lifecycle.js';
import { NKeyMap } from '../../../../base/common/map.js';
import { type IStyledGlyphRasterizer, type IStyledRasterizedGlyph } from '../raster/raster.js';
import { type IStyledReadableTextureAtlasPage, type IStyledTextureAtlasAllocator, type IStyledTextureAtlasPageGlyph } from './atlas.js';
import { TextureAtlasShelfAllocator } from './textureAtlasShelfAllocator.js';
import { TextureAtlasSlabAllocator } from './textureAtlasSlabAllocator.js';

export type AllocatorType = 'shelf' | 'slab' | ((canvas: HTMLCanvasElement, pageIndex: number) => IStyledTextureAtlasAllocator);

export class TextureAtlasPage extends Disposable implements IStyledReadableTextureAtlasPage {
	public static readonly maximumGlyphCount = 2_500;
	public readonly source: HTMLCanvasElement;
	public readonly glyphs = new Set<Readonly<IStyledTextureAtlasPageGlyph>>();
	private readonly allocator: IStyledTextureAtlasAllocator;
	private readonly glyphMap = new NKeyMap<Readonly<IStyledTextureAtlasPageGlyph>, [string, string, string]>();
	private currentVersion = 0;
	private mutableUsedArea = { left: 0, top: 0, right: 0, bottom: 0 };

	constructor(host: HTMLElement, public readonly index: number, pageSize: number, allocatorType: AllocatorType = 'slab') {
		super();
		this.source = h(host.ownerDocument, 'canvas');
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

	public get version(): number { return this.currentVersion; }

	public get usedArea(): Readonly<{ readonly left: number; readonly top: number; readonly right: number; readonly bottom: number }> {
		return this.mutableUsedArea;
	}

	public getGlyph(rasterizer: IStyledGlyphRasterizer, chars: string, styleKey: string, rasterize: () => IStyledRasterizedGlyph): Readonly<IStyledTextureAtlasPageGlyph> | undefined {
		const existing = this.glyphMap.get(rasterizer.cacheKey, styleKey, chars);
		if (existing) return existing;
		if (this.glyphs.size >= TextureAtlasPage.maximumGlyphCount) return undefined;
		const glyph = this.allocator.allocate(rasterize());
		if (!glyph) return undefined;
		this.glyphMap.set(glyph, rasterizer.cacheKey, styleKey, chars);
		this.glyphs.add(glyph);
		this.currentVersion++;
		this.mutableUsedArea = Object.freeze({
			left: 0,
			top: 0,
			right: Math.max(this.mutableUsedArea.right, glyph.x + glyph.w),
			bottom: Math.max(this.mutableUsedArea.bottom, glyph.y + glyph.h),
		});
		return glyph;
	}

	public getUsagePreview(): Promise<Blob> { return this.allocator.getUsagePreview(); }
	public getStats(): string { return this.allocator.getStats(); }
}
