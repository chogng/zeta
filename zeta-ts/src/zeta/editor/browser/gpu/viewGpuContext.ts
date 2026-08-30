import { Emitter, type Event } from '../../../base/common/event.js';
import { Disposable, toDisposable, type IReference } from '../../../base/common/lifecycle.js';
import { PixelRatio, type IPixelRatioMonitor } from '../../../base/browser/pixelRatio.js';
import { h } from '../../../base/browser/dom.js';
import { TextureAtlas } from './atlas/textureAtlas.js';
import { GPULifecycle } from './gpuDisposable.js';
import { observeDevicePixelDimensions } from './gpuUtils.js';
import { DecorationCssRuleExtractor } from './css/decorationCssRuleExtractor.js';
import { DecorationStyleCache } from './css/decorationStyleCache.js';
import { RectangleRenderer } from './rectangleRenderer.js';

type ViewGpuStatus = 'initializing' | 'ready' | 'unavailable';

interface ViewGpuContextOptions {
	readonly host: HTMLElement;
}

const sharedDevices = new WeakMap<Window, Promise<IReference<GPUDevice>>>();

/** Owns the VS Code-aligned WebGPU canvas, device state, DPR, and shared glyph pages for one editor view. */
export class ViewGpuContext extends Disposable {
	public readonly canvas: HTMLCanvasElement;
	public readonly decorationCssRuleExtractor: DecorationCssRuleExtractor;
	public readonly decorationStyleCache = new DecorationStyleCache();
	private readonly ownerWindow: Window;
	private readonly pixelRatio: IPixelRatioMonitor;
	private readonly changeEmitter = this._register(new Emitter<void>());
	public readonly onDidChange: Event<void> = this.changeEmitter.event;
	private canvasContext: GPUCanvasContext | undefined;
	private currentDevice: GPUDevice | undefined;
	private currentAtlas: TextureAtlas | undefined;
	private currentRectangleRenderer: RectangleRenderer | undefined;
	private currentStatus: ViewGpuStatus = 'initializing';
	private currentDevicePixelRatio: number;
	private physicalWidth = 1;
	private physicalHeight = 1;
	private atlasRevision = 0;
	private unavailableReason: Error | undefined;

	constructor(options: ViewGpuContextOptions) {
		super();
		const ownerWindow = options.host.ownerDocument.defaultView;
		if (!ownerWindow) throw new ReferenceError('WebGPU editor rendering requires a browser window');
		this.ownerWindow = ownerWindow;
		this.pixelRatio = PixelRatio.getInstance(ownerWindow);
		this.decorationCssRuleExtractor = this._register(new DecorationCssRuleExtractor());
		this.currentDevicePixelRatio = this.pixelRatio.value;
		this.canvas = h(options.host.ownerDocument, 'canvas');
		this.canvas.className = 'stanza-editor-gpu-canvas';
		this.canvas.setAttribute('aria-hidden', 'true');
		this.canvas.hidden = true;
		options.host.append(this.canvas);
		this._register(toDisposable(() => this.canvas.remove()));
		this._register(this.pixelRatio.onDidChange(value => {
			if (value === this.currentDevicePixelRatio) return;
			this.currentDevicePixelRatio = value;
			if (this.currentDevice) this.createAtlas();
			this.changeEmitter.fire();
		}));
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

	public get rectangleRenderer(): RectangleRenderer {
		if (!this.currentRectangleRenderer) throw new Error('WebGPU rectangle renderer is not ready');
		return this.currentRectangleRenderer;
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

	public get failure(): Error | undefined {
		return this.unavailableReason;
	}

	public layout(width: number, height: number, left: number, top: number): void {
		if (![width, height, left, top].every(Number.isFinite) || width < 0 || height < 0 || left < 0 || top < 0) {
			throw new RangeError('WebGPU editor layout values must be finite and non-negative');
		}
		this.canvas.style.width = `${width}px`;
		this.canvas.style.height = `${height}px`;
		this.canvas.style.left = `${left}px`;
		this.canvas.style.top = `${top}px`;
		const nextDevicePixelRatio = this.pixelRatio.value;
		if (nextDevicePixelRatio !== this.currentDevicePixelRatio) {
			this.currentDevicePixelRatio = nextDevicePixelRatio;
			if (this.currentDevice) this.createAtlas();
		}
		this.setPhysicalSize(Math.max(1, Math.ceil(width * this.currentDevicePixelRatio)), Math.max(1, Math.ceil(height * this.currentDevicePixelRatio)));
	}

	public clearAtlas(): void {
		if (!this.currentAtlas) return;
		this.decorationCssRuleExtractor.clear();
		this.currentAtlas.clear();
		this.atlasRevision += 1;
		this.changeEmitter.fire();
	}

	public markUnavailable(error: Error): void {
		if (this.currentStatus === 'unavailable') return;
		this.currentStatus = 'unavailable';
		this.unavailableReason = error;
		this.hideCanvas();
		this.changeEmitter.fire();
	}

	public showCanvas(): void {
		if (this.currentStatus !== 'ready') throw new Error('WebGPU canvas cannot be shown before the device is ready');
		this.canvas.hidden = false;
	}

	public hideCanvas(): void {
		this.canvas.hidden = true;
	}

	private async initialize(): Promise<void> {
		try {
			let devicePromise = sharedDevices.get(this.ownerWindow);
			if (!devicePromise) {
				devicePromise = GPULifecycle.requestDevice(this.ownerWindow);
				sharedDevices.set(this.ownerWindow, devicePromise);
				this.ownerWindow.addEventListener('pagehide', () => void devicePromise?.then(reference => reference.dispose()), { once: true });
			}
			const reference = await devicePromise;
			if (this.isDisposed) return;
			const canvasContext = this.canvas.getContext('webgpu');
			if (!canvasContext) throw new Error('This browser cannot create a WebGPU canvas context');
			this.currentDevice = reference.object;
			this.canvasContext = canvasContext;
			const presentationFormat = this.ownerWindow.navigator.gpu.getPreferredCanvasFormat();
			canvasContext.configure({
				device: reference.object,
				format: presentationFormat,
				alphaMode: 'premultiplied',
			});
			this.currentRectangleRenderer = this._register(new RectangleRenderer(reference.object, presentationFormat));
			this.createAtlas();
			this.currentStatus = 'ready';
			void reference.object.lost.then(info => this.markUnavailable(new Error(`WebGPU device lost: ${info.message || info.reason}`)));
			this.changeEmitter.fire();
		} catch (error) {
			if (!this.isDisposed) this.markUnavailable(asError(error));
		}
	}

	private createAtlas(): void {
		const maximumTextureSize = this.device.limits.maxTextureDimension2D;
		const pageSize = Math.min(maximumTextureSize, 1024 * Math.max(1, Math.floor(this.currentDevicePixelRatio)));
		this.currentAtlas?.dispose();
		this.currentAtlas = new TextureAtlas(this.canvas, pageSize);
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
