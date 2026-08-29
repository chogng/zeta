import { type GpuRenderFrame, type GpuRenderStrategyInput } from '../gpu.js';
import { type GlyphRasterizer } from '../raster/glyphRasterizer.js';
import { createGpuRenderFrame } from '../gpuUtils.js';
import { type EditorVisualLineProjection } from '../../../common/viewModel/modelLineProjection.js';
import { BaseRenderStrategy } from './baseRenderStrategy.js';
import { fullFileRenderStrategyWgsl } from './fullFileRenderStrategy.wgsl.js';

export class FullFileRenderStrategy extends BaseRenderStrategy {
	public static readonly maxSupportedLines = 3_000;
	public static readonly maxSupportedColumns = 200;
	public readonly type = 'fullfile';
	public readonly wgsl = fullFileRenderStrategyWgsl;
	public readonly bindGroupEntries: readonly GPUBindGroupEntry[] = Object.freeze([]);
	private cacheKey: string | undefined;
	private cachedProjection: EditorVisualLineProjection | undefined;
	private cachedVertices: Float32Array<ArrayBuffer> | undefined;
	private cachedGpuLineIndexes: ReadonlySet<number> = new Set();

	constructor(glyphRasterizer: GlyphRasterizer) { super(glyphRasterizer); }

	public reset(): void {
		this.cacheKey = undefined;
		this.cachedProjection = undefined;
		this.cachedVertices = undefined;
		this.cachedGpuLineIndexes = new Set();
	}

	public draw(pass: GPURenderPassEncoder, frame: GpuRenderFrame): void {
		pass.draw(frame.vertices.length / 5);
	}

	public update(input: GpuRenderStrategyInput): GpuRenderFrame {
		const key = createCacheKey(input);
		if (key !== this.cacheKey || input.visualLines !== this.cachedProjection || !this.cachedVertices) {
			const allLineIndexes = new Set(Array.from({ length: input.visualLines.visualLineCount }, (_, index) => index));
			const completeFrame = createGpuRenderFrame(this.glyphRasterizer, { ...input, visibleLineIndexes: allLineIndexes }, allLineIndexes);
			this.cacheKey = key;
			this.cachedProjection = input.visualLines;
			this.cachedVertices = completeFrame.vertices;
			this.cachedGpuLineIndexes = completeFrame.gpuLineIndexes;
		}
		const gpuLineIndexes = new Set([...input.visibleLineIndexes].filter(index => this.cachedGpuLineIndexes.has(index)));
		return Object.freeze({ vertices: this.cachedVertices, gpuLineIndexes });
	}
}

function createCacheKey(input: GpuRenderStrategyInput): string {
	const style = input.rootStyle;
	return [
		input.visualLines.modelVersion,
		input.visualLines.visualLineCount,
		input.layout.lineHeight,
		input.textLeft,
		input.paddingTop,
		input.textDirection,
		input.fontLigatures,
		style.color,
		style.fontFamily,
		style.fontSize,
		style.fontStyle,
		style.fontVariant,
		style.fontWeight,
		style.letterSpacing,
		style.tabSize,
	].join('|');
}
