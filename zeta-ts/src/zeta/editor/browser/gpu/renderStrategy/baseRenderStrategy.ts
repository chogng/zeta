import { Disposable } from '../../../../base/common/lifecycle.js';
import { type GpuFrame, type GpuRenderInput, type IGpuRenderStrategy } from '../gpu.js';

export abstract class BaseRenderStrategy extends Disposable implements IGpuRenderStrategy {
	public abstract readonly type: string;
	public abstract readonly wgsl: string;
	public abstract readonly bindGroupEntries: readonly GPUBindGroupEntry[];

	constructor(public readonly glyphRasterizer: IGpuRenderStrategy['glyphRasterizer']) {
		super();
	}

	public abstract reset(): void;
	public abstract update(input: GpuRenderInput): GpuFrame;
	public abstract draw(pass: GPURenderPassEncoder, frame: GpuFrame): void;
}
