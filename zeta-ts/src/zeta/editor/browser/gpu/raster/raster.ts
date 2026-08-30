import { type MetadataConsts } from '../../../common/encodedTokenAttributes.js';

export interface IGlyphRasterizer {
	readonly id: number;
	readonly cacheKey: string;
	rasterizeGlyph(chars: string, tokenMetadata: number, decorationStyleSetId: number, colorMap: string[]): Readonly<IRasterizedGlyph>;
	getTextMetrics(text: string): TextMetrics;
}

export interface IBoundingBox {
	readonly left: number;
	readonly top: number;
	readonly right: number;
	readonly bottom: number;
}

export interface IRasterizedGlyph {
	readonly source: OffscreenCanvas;
	readonly boundingBox: IBoundingBox;
	readonly originOffset: { x: number; y: number };
	readonly fontBoundingBoxAscent: number;
	readonly fontBoundingBoxDescent: number;
}

export interface IGpuGlyphStyle {
	readonly color: string;
	readonly fontFamily: string;
	readonly fontSize: number;
	readonly fontStyle: string;
	readonly fontVariant: string;
	readonly fontWeight: string;
	readonly letterSpacing: number;
}

/** Browser-canvas rasterizer contract used by Zeta's current GPU renderer. */
export interface IStyledGlyphRasterizer {
	readonly id: number;
	readonly cacheKey: string;
	readonly devicePixelRatio: number;
	styleKey(style: IGpuGlyphStyle): string;
	rasterizeGlyph(chars: string, style: IGpuGlyphStyle, subPixelX: number): Readonly<IStyledRasterizedGlyph>;
	getTextMetrics(text: string, style: IGpuGlyphStyle): TextMetrics;
}

export interface IStyledRasterizedGlyph {
	readonly source: HTMLCanvasElement;
	readonly boundingBox: IBoundingBox;
	readonly originOffset: { readonly x: number; readonly y: number };
	readonly advance: number;
	readonly fontBoundingBoxAscent: number;
	readonly fontBoundingBoxDescent: number;
}
