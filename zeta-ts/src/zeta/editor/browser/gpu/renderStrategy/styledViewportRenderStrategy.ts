import { type GpuRenderFrame, type GpuRenderStrategyInput } from '../gpuFrameStrategy.js';
import { type StyledGlyphRasterizer } from '../raster/styledGlyphRasterizer.js';
import { createGpuRenderFrame } from '../gpuUtils.js';
import { StyledBaseRenderStrategy } from './styledBaseRenderStrategy.js';
import { fullFileRenderStrategyWgsl } from './fullFileRenderStrategy.wgsl.js';

export class StyledViewportRenderStrategy extends StyledBaseRenderStrategy {
	public static readonly maxSupportedColumns = 2_000;
	public readonly type = 'viewport';
	public readonly wgsl = fullFileRenderStrategyWgsl;
	public readonly bindGroupEntries: readonly GPUBindGroupEntry[] = Object.freeze([]);

	constructor(glyphRasterizer: StyledGlyphRasterizer) { super(glyphRasterizer); }

	public reset(): void {}

	public draw(pass: GPURenderPassEncoder, frame: GpuRenderFrame): void {
		pass.draw(frame.vertices.length / 5);
	}

	public update(input: GpuRenderStrategyInput): GpuRenderFrame {
		return createGpuRenderFrame(this.glyphRasterizer, input, input.visibleLineIndexes);
	}
}
