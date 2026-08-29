import { Emitter, type Event } from '../../../../base/common/event.js';
import { Disposable, dispose, toDisposable } from '../../../../base/common/lifecycle.js';
import { type IGlyphRasterizer, type IGpuGlyphStyle } from '../raster/raster.js';
import { type IReadableTextureAtlasPage, type ITextureAtlasPageGlyph } from './atlas.js';
import { type AllocatorType, TextureAtlasPage } from './textureAtlasPage.js';

export interface ITextureAtlasOptions {
	readonly allocatorType?: AllocatorType;
}

export class TextureAtlas extends Disposable {
	public static readonly maximumPageCount = 16;
	private readonly mutablePages: TextureAtlasPage[] = [];
	private readonly deleteGlyphsEmitter = this._register(new Emitter<void>());
	public readonly onDidDeleteGlyphs: Event<void> = this.deleteGlyphsEmitter.event;

	constructor(private readonly host: HTMLElement, public readonly pageSize: number, private readonly options: ITextureAtlasOptions = {}) {
		super();
		if (!Number.isSafeInteger(pageSize) || pageSize < 64) throw new RangeError('WebGPU texture atlas page size must be an integer of at least 64 pixels');
		this.mutablePages.push(this.createPage(0));
		this._register(toDisposable(() => {
			dispose(this.mutablePages);
			this.mutablePages.length = 0;
		}));
	}

	public get pages(): readonly IReadableTextureAtlasPage[] {
		return this.mutablePages;
	}

	public getGlyph(rasterizer: IGlyphRasterizer, chars: string, style: IGpuGlyphStyle, deviceX: number): Readonly<ITextureAtlasPageGlyph> {
		const subPixelBucket = Math.round((deviceX - Math.floor(deviceX)) * 10) % 10;
		const styleKey = `${rasterizer.styleKey(style)}|${subPixelBucket}`;
		for (const page of this.mutablePages) {
			const glyph = page.getGlyph(rasterizer, chars, styleKey, () => rasterizer.rasterizeGlyph(chars, style, subPixelBucket / 10));
			if (glyph) return glyph;
		}
		if (this.mutablePages.length >= TextureAtlas.maximumPageCount) throw new RangeError('WebGPU texture atlas exhausted its page limit');
		const page = this.createPage(this.mutablePages.length);
		this.mutablePages.push(page);
		const glyph = page.getGlyph(rasterizer, chars, styleKey, () => rasterizer.rasterizeGlyph(chars, style, subPixelBucket / 10));
		if (!glyph) throw new RangeError('WebGPU glyph exceeds the texture atlas page size');
		return glyph;
	}

	public clear(): void {
		dispose(this.mutablePages);
		this.mutablePages.splice(0, this.mutablePages.length, this.createPage(0));
		this.deleteGlyphsEmitter.fire();
	}

	private createPage(index: number): TextureAtlasPage {
		return new TextureAtlasPage(this.host, index, this.pageSize, this.options.allocatorType);
	}
}
