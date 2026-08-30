import { type GpuFrame, type GpuRenderInput } from '../gpu.js';
import { createGpuRenderFrame } from '../gpuUtils.js';
import { type GlyphRasterizer } from '../raster/glyphRasterizer.js';
import { BaseRenderStrategy } from './baseRenderStrategy.js';
import { fullFileRenderStrategyWgsl } from './fullFileRenderStrategy.wgsl.js';

export class ViewportRenderStrategy extends BaseRenderStrategy {
	public static readonly maxSupportedColumns = 2_000;
	public readonly type = 'viewport';
	public readonly wgsl = fullFileRenderStrategyWgsl;
	public readonly bindGroupEntries: readonly GPUBindGroupEntry[] = Object.freeze([]);

	constructor(glyphRasterizer: GlyphRasterizer) { super(glyphRasterizer); }

	public reset(): void { }

	public draw(pass: GPURenderPassEncoder, frame: GpuFrame): void {
		pass.draw(frame.vertices.length / 5);
	}

	public update(input: GpuRenderInput): GpuFrame {
		return createGpuRenderFrame(this.glyphRasterizer, input, input.visibleLineIndexes);
	}
}
