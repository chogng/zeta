import { type GpuRenderFrame, type GpuRenderStrategyInput } from '../gpu.js';
import { type GlyphRasterizer } from '../raster/glyphRasterizer.js';
import { BaseRenderStrategy } from './baseRenderStrategy.js';

export class ViewportRenderStrategy extends BaseRenderStrategy {
	public static readonly maxSupportedColumns = 2_000;
	public readonly type = 'viewport';

	constructor(glyphRasterizer: GlyphRasterizer) { super(glyphRasterizer); }

	public reset(): void {}

	public update(input: GpuRenderStrategyInput): GpuRenderFrame {
		return this.createFrame(input, input.visibleLineIndexes);
	}
}
