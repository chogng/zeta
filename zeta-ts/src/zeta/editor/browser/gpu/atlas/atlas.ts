import { type NKeyMap } from '../../../../base/common/map.js';
import { type IBoundingBox, type IRasterizedGlyph, type IStyledGlyphRasterizer, type IStyledRasterizedGlyph } from '../raster/raster.js';

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

/** Glyph metadata used by the CSS-style GPU renderer. */
export interface IStyledTextureAtlasPageGlyph extends ITextureAtlasPageGlyph {
	readonly advance: number;
}

export interface ITextureAtlasAllocator {
	allocate(rasterizedGlyph: Readonly<IRasterizedGlyph>): Readonly<ITextureAtlasPageGlyph> | undefined;
	getUsagePreview(): Promise<Blob>;
	getStats(): string;
}

export interface IStyledTextureAtlasAllocator {
	allocate(rasterizedGlyph: Readonly<IStyledRasterizedGlyph>): Readonly<IStyledTextureAtlasPageGlyph> | undefined;
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

/** Browser-canvas atlas page used by the CSS-style GPU renderer. */
export interface IStyledReadableTextureAtlasPage {
	readonly index: number;
	readonly source: HTMLCanvasElement;
	readonly version: number;
	readonly glyphs: ReadonlySet<Readonly<IStyledTextureAtlasPageGlyph>>;
	readonly usedArea: Readonly<IBoundingBox>;
	getGlyph(rasterizer: IStyledGlyphRasterizer, chars: string, styleKey: string, rasterize: () => IStyledRasterizedGlyph): Readonly<IStyledTextureAtlasPageGlyph> | undefined;
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
