import { Emitter, type Event } from '../../../base/common/event.js';
import { Disposable, toDisposable } from '../../../base/common/lifecycle.js';
import { TextureAtlas } from './atlas/textureAtlas.js';
import { GpuLifecycle, type GpuDeviceReference } from './gpuDisposable.js';
import { observeDevicePixelDimensions, validatedDevicePixelRatio } from './gpuUtils.js';

export type ViewGpuStatus = 'initializing' | 'ready' | 'unavailable';

export interface ViewGpuContextOptions {
	readonly host: HTMLElement;
	readonly onError: (error: Error) => void;
}

const sharedDevices = new WeakMap<Window, Promise<GpuDeviceReference>>();

/** Owns the VS Code-aligned WebGPU canvas, device state, DPR, and shared glyph pages for one editor view. */
export class ViewGpuContext extends Disposable {
	public readonly canvas: HTMLCanvasElement;
	private readonly ownerWindow: Window;
	private readonly onError: (error: Error) => void;
	private readonly changeEmitter = this._register(new Emitter<void>());
	public readonly onDidChange: Event<void> = this.changeEmitter.event;
	private canvasContext: GPUCanvasContext | undefined;
	private currentDevice: GPUDevice | undefined;
	private currentAtlas: TextureAtlas | undefined;
	private currentStatus: ViewGpuStatus = 'initializing';
	private currentDevicePixelRatio: number;
	private physicalWidth = 1;
	private physicalHeight = 1;
	private atlasRevision = 0;

	constructor(options: ViewGpuContextOptions) {
		super();
		this.onError = options.onError;
		const ownerWindow = options.host.ownerDocument.defaultView;
		if (!ownerWindow) throw new ReferenceError('WebGPU editor rendering requires a browser window');
		this.ownerWindow = ownerWindow;
		this.currentDevicePixelRatio = validatedDevicePixelRatio(ownerWindow);
		this.canvas = options.host.ownerDocument.createElement('canvas');
		this.canvas.className = 'stanza-editor-gpu-canvas';
		this.canvas.setAttribute('aria-hidden', 'true');
		this.canvas.hidden = true;
		options.host.append(this.canvas);
		this._register(toDisposable(() => this.canvas.remove()));
		try {
			this._register(observeDevicePixelDimensions(this.canvas, ownerWindow, (width, height) => this.setPhysicalSize(width, height)));
		} catch (error) {
			this.markUnavailable(asError(error));
			return;
		}
		void this.initialize();
	}

	public get status(): ViewGpuStatus {
		return this.currentStatus;
	}

	public get device(): GPUDevice {
		if (!this.currentDevice) throw new Error('WebGPU editor device is not ready');
		return this.currentDevice;
	}

	public get context(): GPUCanvasContext {
		if (!this.canvasContext) throw new Error('WebGPU editor canvas context is not ready');
		return this.canvasContext;
	}

	public get atlas(): TextureAtlas {
		if (!this.currentAtlas) throw new Error('WebGPU editor texture atlas is not ready');
		return this.currentAtlas;
	}

	public get devicePixelRatio(): number {
		return this.currentDevicePixelRatio;
	}

	public get devicePixelDimensions(): { readonly width: number; readonly height: number } {
		return Object.freeze({ width: this.physicalWidth, height: this.physicalHeight });
	}

	public get textureAtlasRevision(): number {
		return this.atlasRevision;
	}

	public layout(width: number, height: number, left: number, top: number): void {
		if (![width, height, left, top].every(Number.isFinite) || width < 0 || height < 0 || left < 0 || top < 0) {
			throw new RangeError('WebGPU editor layout values must be finite and non-negative');
		}
		this.canvas.style.width = `${width}px`;
		this.canvas.style.height = `${height}px`;
		this.canvas.style.left = `${left}px`;
		this.canvas.style.top = `${top}px`;
		const nextDevicePixelRatio = validatedDevicePixelRatio(this.ownerWindow);
		if (nextDevicePixelRatio !== this.currentDevicePixelRatio) {
			this.currentDevicePixelRatio = nextDevicePixelRatio;
			if (this.currentDevice) this.createAtlas();
		}
		this.setPhysicalSize(Math.max(1, Math.ceil(width * this.currentDevicePixelRatio)), Math.max(1, Math.ceil(height * this.currentDevicePixelRatio)));
	}

	public clearAtlas(): void {
		if (!this.currentAtlas) return;
		this.currentAtlas.clear();
		this.atlasRevision += 1;
		this.changeEmitter.fire();
	}

	public markUnavailable(error: Error): void {
		if (this.currentStatus === 'unavailable') return;
		this.currentStatus = 'unavailable';
		this.canvas.hidden = true;
		this.onError(error);
		this.changeEmitter.fire();
	}

	private async initialize(): Promise<void> {
		try {
			let devicePromise = sharedDevices.get(this.ownerWindow);
			if (!devicePromise) {
				devicePromise = GpuLifecycle.requestDevice(this.ownerWindow);
				sharedDevices.set(this.ownerWindow, devicePromise);
				this.ownerWindow.addEventListener('pagehide', () => void devicePromise?.then(reference => reference.dispose()), { once: true });
			}
			const reference = await devicePromise;
			if (this.isDisposed) return;
			const canvasContext = this.canvas.getContext('webgpu');
			if (!canvasContext) throw new Error('This browser cannot create a WebGPU canvas context');
			this.currentDevice = reference.device;
			this.canvasContext = canvasContext;
			canvasContext.configure({
				device: reference.device,
				format: this.ownerWindow.navigator.gpu.getPreferredCanvasFormat(),
				alphaMode: 'premultiplied',
			});
			this.createAtlas();
			this.currentStatus = 'ready';
			this.canvas.hidden = false;
			void reference.device.lost.then(info => this.markUnavailable(new Error(`WebGPU device lost: ${info.message || info.reason}`)));
			this.changeEmitter.fire();
		} catch (error) {
			if (!this.isDisposed) this.markUnavailable(asError(error));
		}
	}

	private createAtlas(): void {
		const maximumTextureSize = this.device.limits.maxTextureDimension2D;
		const pageSize = Math.min(maximumTextureSize, 1024 * Math.max(1, Math.floor(this.currentDevicePixelRatio)));
		this.currentAtlas = new TextureAtlas(this.canvas.ownerDocument, pageSize);
		this.atlasRevision += 1;
	}

	private setPhysicalSize(width: number, height: number): void {
		if (!Number.isSafeInteger(width) || !Number.isSafeInteger(height) || width < 1 || height < 1) return;
		if (this.physicalWidth === width && this.physicalHeight === height) return;
		this.physicalWidth = width;
		this.physicalHeight = height;
		this.canvas.width = width;
		this.canvas.height = height;
		this.changeEmitter.fire();
	}
}

function asError(error: unknown): Error {
	return error instanceof Error ? error : new Error(String(error));
}
