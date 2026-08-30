import { Emitter, type Event } from '../../../../base/common/event.js';
import { Disposable, dispose, MutableDisposable, toDisposable } from '../../../../base/common/lifecycle.js';
import { type ITaskQueue } from '../taskQueue.js';
import { WindowIdleTaskQueue } from '../windowIdleTaskQueue.js';
import { type IGpuGlyphStyle, type IStyledGlyphRasterizer } from '../raster/raster.js';
import { type IGpuReadableTextureAtlasPage, type IGpuTextureAtlasPageGlyph } from './atlas.js';
import { type AllocatorType, type GpuAllocatorType, TextureAtlasPage } from './textureAtlasPage.js';

export interface ITextureAtlasOptions {
	readonly allocatorType: AllocatorType;
}

export interface IGpuTextureAtlasOptions {
	readonly allocatorType?: GpuAllocatorType;
}

export class TextureAtlas extends Disposable {
	public static readonly maximumPageCount = 16;
	private readonly mutablePages: TextureAtlasPage[] = [];
	private readonly warmUpTask = this._register(new MutableDisposable<ITaskQueue>());
	private warmedRasterizers = new WeakSet<IStyledGlyphRasterizer>();
	private readonly deleteGlyphsEmitter = this._register(new Emitter<void>());
	public readonly onDidDeleteGlyphs: Event<void> = this.deleteGlyphsEmitter.event;

	constructor(private readonly host: HTMLElement, public readonly pageSize: number, private readonly options: IGpuTextureAtlasOptions = {}) {
		super();
		if (!Number.isSafeInteger(pageSize) || pageSize < 64) throw new RangeError('WebGPU texture atlas page size must be an integer of at least 64 pixels');
		this.mutablePages.push(this.createPage(0));
		this._register(toDisposable(() => {
			dispose(this.mutablePages);
			this.mutablePages.length = 0;
		}));
	}

	public get pages(): readonly IGpuReadableTextureAtlasPage[] {
		return this.mutablePages;
	}

	public getGlyph(rasterizer: IStyledGlyphRasterizer, chars: string, style: IGpuGlyphStyle, deviceX: number): Readonly<IGpuTextureAtlasPageGlyph> {
		if (!this.warmedRasterizers.has(rasterizer)) {
			this.warmedRasterizers.add(rasterizer);
			this.warmUp(rasterizer, style);
		}
		const subPixelBucket = Math.round((deviceX - Math.floor(deviceX)) * 10) % 10;
		const styleKey = JSON.stringify([rasterizer.styleKey(style), subPixelBucket]);
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
		this.warmUpTask.clear();
		this.warmedRasterizers = new WeakSet();
		dispose(this.mutablePages);
		this.mutablePages.splice(0, this.mutablePages.length, this.createPage(0));
		this.deleteGlyphsEmitter.fire();
	}

	private createPage(index: number): TextureAtlasPage {
		return new TextureAtlasPage(this.host, index, this.pageSize, this.options.allocatorType);
	}

	private warmUp(rasterizer: IStyledGlyphRasterizer, style: IGpuGlyphStyle): void {
		const ownerWindow = this.host.ownerDocument.defaultView;
		if (!ownerWindow) return;
		this.warmUpTask.value?.clear();
		const queue = this.warmUpTask.value = new WindowIdleTaskQueue(ownerWindow);
		for (let code = 33; code <= 126; code += 1) {
			queue.enqueue(() => {
				if (this.isDisposed) return;
				this.getGlyph(rasterizer, String.fromCharCode(code), style, 0);
			});
		}
	}
}
