import { toDisposable, type IDisposable } from '../../../base/common/lifecycle.js';

export interface GpuDeviceReference extends IDisposable {
	readonly device: GPUDevice;
}

/** Owns WebGPU resources whose API exposes explicit destruction. */
export namespace GpuLifecycle {
	export async function requestDevice(ownerWindow: Window): Promise<GpuDeviceReference> {
		const gpu = ownerWindow.navigator.gpu;
		if (!gpu) throw new Error('This browser does not support WebGPU');
		const adapter = await gpu.requestAdapter();
		if (!adapter) throw new Error('WebGPU is disabled or no compatible adapter is available');
		const device = await adapter.requestDevice();
		return Object.assign(toDisposable(() => device.destroy()), { device });
	}

	export function createBuffer(device: GPUDevice, descriptor: GPUBufferDescriptor): GpuResourceReference<GPUBuffer> {
		return destroyable(device.createBuffer(descriptor));
	}

	export function createTexture(device: GPUDevice, descriptor: GPUTextureDescriptor): GpuResourceReference<GPUTexture> {
		return destroyable(device.createTexture(descriptor));
	}
}

export interface GpuResourceReference<T> extends IDisposable {
	readonly object: T;
}

function destroyable<T extends { destroy(): void }>(object: T): GpuResourceReference<T> {
	return Object.assign(toDisposable(() => object.destroy()), { object });
}
