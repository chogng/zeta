import { type IDisposable } from '../../../base/common/lifecycle.js';
import { type TextMeasurer } from '../config/fontMeasurements.js';
import { type EditorTextDirection } from '../view.js';
import { type EditorViewportLayout } from '../../common/viewLayout/viewLayout.js';
import { type EditorVisualLineProjection } from '../../common/viewModel/modelLineProjection.js';
import { type TextModel } from '../../common/model/textModel.js';
import { type BracketColorizationSource, type SemanticTokenSource } from '../viewparts/semanticTokens/semanticTokenPresentation.js';
import { type TextureAtlas } from './atlas/textureAtlas.js';
import { type GlyphRasterizer } from './raster/glyphRasterizer.js';

export const enum BindingId {
	GlyphInfo,
	Cells,
	TextureSampler,
	Texture,
	LayoutInfoUniform,
	AtlasDimensionsUniform,
	ScrollOffset,
}

export interface GpuRenderStrategyInput {
	readonly layout: EditorViewportLayout;
	readonly model: TextModel;
	readonly visualLines: EditorVisualLineProjection;
	readonly visibleLineIndexes: ReadonlySet<number>;
	readonly semanticTokenSource: SemanticTokenSource | undefined;
	readonly bracketColorizationSource: BracketColorizationSource | undefined;
	readonly textMeasurer: TextMeasurer;
	readonly textLeft: number;
	readonly paddingTop: number;
	readonly textDirection: EditorTextDirection;
	readonly fontLigatures: boolean;
	readonly rootStyle: CSSStyleDeclaration;
	readonly atlas: TextureAtlas;
}

export interface GpuRenderFrame {
	readonly vertices: Float32Array<ArrayBuffer>;
	readonly gpuLineIndexes: ReadonlySet<number>;
}

export interface IGpuRenderStrategy extends IDisposable {
	readonly type: string;
	readonly wgsl: string;
	readonly bindGroupEntries: readonly GPUBindGroupEntry[];
	readonly glyphRasterizer: GlyphRasterizer;
	reset(): void;
	update(input: GpuRenderStrategyInput): GpuRenderFrame;
	draw(pass: GPURenderPassEncoder, frame: GpuRenderFrame): void;
}
