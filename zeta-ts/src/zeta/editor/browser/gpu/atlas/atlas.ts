import { type NKeyMap } from '../../../../base/common/map.js';
import { type IBoundingBox, type IGlyphRasterizer, type IRasterizedGlyph, type IStyledGlyphRasterizer, type IStyledRasterizedGlyph } from '../raster/raster.js';

export interface ITextureAtlasPageGlyph {
	readonly pageIndex: number;
	readonly glyphIndex: number;
	readonly x: number;
	readonly y: number;
	readonly w: number;
	readonly h: number;
	readonly originOffsetX: number;
	readonly originOffsetY: number;
	readonly fontBoundingBoxAscent: number;
	readonly fontBoundingBoxDescent: number;
}

/** Zeta GPU layout metadata kept separate from the canonical atlas glyph contract. */
export interface IGpuTextureAtlasPageGlyph extends ITextureAtlasPageGlyph {
	readonly advance: number;
}

export interface ITextureAtlasAllocator {
	allocate(rasterizedGlyph: Readonly<IRasterizedGlyph>): Readonly<ITextureAtlasPageGlyph> | undefined;
	getUsagePreview(): Promise<Blob>;
	getStats(): string;
}

export interface IGpuTextureAtlasAllocator {
	allocate(rasterizedGlyph: Readonly<IStyledRasterizedGlyph>): Readonly<IGpuTextureAtlasPageGlyph> | undefined;
	getUsagePreview(): Promise<Blob>;
	getStats(): string;
}

export interface IReadableTextureAtlasPage {
	readonly version: number;
	readonly usedArea: Readonly<IBoundingBox>;
	readonly glyphs: IterableIterator<Readonly<ITextureAtlasPageGlyph>>;
	readonly source: OffscreenCanvas;
	getUsagePreview(): Promise<Blob>;
	getStats(): string;
}

/** Browser-canvas atlas page used by Zeta's current GPU renderer. */
export interface IGpuReadableTextureAtlasPage {
	readonly index: number;
	readonly source: HTMLCanvasElement;
	readonly version: number;
	readonly glyphs: ReadonlySet<Readonly<IGpuTextureAtlasPageGlyph>>;
	readonly usedArea: Readonly<IBoundingBox>;
	getGlyph(rasterizer: IStyledGlyphRasterizer, chars: string, styleKey: string, rasterize: () => IStyledRasterizedGlyph): Readonly<IGpuTextureAtlasPageGlyph> | undefined;
	getUsagePreview(): Promise<Blob>;
	getStats(): string;
}

export const enum UsagePreviewColors {
	Unused = '#808080',
	Used = '#4040FF',
	Wasted = '#FF0000',
	Restricted = '#FF000088',
}

export type GlyphMap<T> = NKeyMap<T, [
	chars: string,
	tokenMetadata: number,
	decorationStyleSetId: number,
	rasterizerCacheKey: string,
]>;
