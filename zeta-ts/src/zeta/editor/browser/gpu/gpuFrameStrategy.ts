import { type IDisposable } from '../../../base/common/lifecycle.js';
import { type EditorViewportLayout } from '../../common/viewLayout/viewLayout.js';
import { type EditorVisualLineProjection } from '../../common/viewModel/modelLineProjection.js';
import { type TextModel } from '../../common/model/textModel.js';
import { type BracketColorizationSource, type SemanticTokenSource } from '../viewParts/viewLines/viewLine.js';
import { type StyledTextureAtlas } from './atlas/styledTextureAtlas.js';
import { type StyledGlyphRasterizer } from './raster/styledGlyphRasterizer.js';
import { type EditorTextDirection } from '../viewParts/viewLines/viewLineOptions.js';

export interface GpuRenderStrategyInput {
	readonly layout: EditorViewportLayout;
	readonly model: TextModel;
	readonly visualLines: EditorVisualLineProjection;
	readonly visibleLineIndexes: ReadonlySet<number>;
	readonly semanticTokenSource: SemanticTokenSource | undefined;
	readonly bracketColorizationSource: BracketColorizationSource | undefined;
	readonly textLeft: number;
	readonly paddingTop: number;
	readonly textDirection: EditorTextDirection;
	readonly fontLigatures: boolean;
	readonly rootStyle: CSSStyleDeclaration;
	readonly atlas: StyledTextureAtlas;
}

export interface GpuRenderFrame {
	readonly vertices: Float32Array<ArrayBuffer>;
	readonly gpuLineIndexes: ReadonlySet<number>;
}

export interface IGpuFrameRenderStrategy extends IDisposable {
	readonly type: string;
	readonly wgsl: string;
	readonly bindGroupEntries: readonly GPUBindGroupEntry[];
	readonly glyphRasterizer: StyledGlyphRasterizer;
	reset(): void;
	update(input: GpuRenderStrategyInput): GpuRenderFrame;
	draw(pass: GPURenderPassEncoder, frame: GpuRenderFrame): void;
}
