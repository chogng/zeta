import { type IBoundingBox, type IGlyphRasterizer, type IRasterizedGlyph } from '../raster/raster.js';

export interface ITextureAtlasPageGlyph {
	readonly pageIndex: number;
	readonly glyphIndex: number;
	readonly x: number;
	readonly y: number;
	readonly w: number;
	readonly h: number;
	readonly originOffsetX: number;
	readonly originOffsetY: number;
	readonly advance: number;
	readonly fontBoundingBoxAscent: number;
	readonly fontBoundingBoxDescent: number;
}

export interface ITextureAtlasAllocator {
	allocate(rasterizedGlyph: IRasterizedGlyph): ITextureAtlasPageGlyph | undefined;
	getUsagePreview(): Promise<Blob>;
	getStats(): string;
}

export interface IReadableTextureAtlasPage {
	readonly index: number;
	readonly source: HTMLCanvasElement;
	readonly version: number;
	readonly glyphs: ReadonlySet<Readonly<ITextureAtlasPageGlyph>>;
	readonly usedArea: Readonly<IBoundingBox>;
	getGlyph(rasterizer: IGlyphRasterizer, chars: string, styleKey: string, rasterize: () => IRasterizedGlyph): Readonly<ITextureAtlasPageGlyph> | undefined;
}

export const enum UsagePreviewColors {
	Unused = '#808080',
	Used = '#00ff00',
	Wasted = '#ff0000',
}

export type GlyphMap<T> = Map<string, T>;
