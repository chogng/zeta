import { Disposable, MutableDisposable, type IReference } from '../../../../base/common/lifecycle.js';
import { ViewGpuContext } from '../../gpu/viewGpuContext.js';
import { GPULifecycle } from '../../gpu/gpuDisposable.js';
import { GlyphRasterizer } from '../../gpu/raster/glyphRasterizer.js';
import { type EditorVisualLineProjection } from '../../../common/viewModel/modelLineProjection.js';
import { type EditorViewportLayout } from '../../../common/viewLayout/viewLayout.js';
import { type TextModel } from '../../../common/model/textModel.js';
import { type BracketColorizationSource, type SemanticTokenSource } from '../viewLines/semanticTokenPresentation.js';
import { type ViewLine } from '../viewLines/viewLine.js';
import { type ViewLineOptions } from '../viewLines/viewLineOptions.js';
import { type ViewLines } from '../viewLines/viewLines.js';
import { BindingId, type GpuRenderFrame, type IGpuRenderStrategy } from '../../gpu/gpu.js';
import { FullFileRenderStrategy } from '../../gpu/renderStrategy/fullFileRenderStrategy.js';
import { ViewportRenderStrategy } from '../../gpu/renderStrategy/viewportRenderStrategy.js';
import { type EditorRenderingContext } from '../../view/renderingContext.js';

export interface ViewLinesGpuOptions {
	readonly host: HTMLElement;
	readonly model: TextModel;
	readonly semanticTokenSource: SemanticTokenSource | undefined;
	readonly bracketColorizationSource: BracketColorizationSource | undefined;
	readonly paddingTop: number;
	readonly viewLineOptions: ViewLineOptions;
	readonly viewLines: ViewLines;
}

interface PreparedGpuFrame {
	readonly frame: GpuRenderFrame;
	readonly renderedLines: ReadonlyMap<number, ViewLine>;
}

const VERTEX_FLOAT_COUNT = 5;

/** Draws eligible visible text rows through the VS Code-aligned WebGPU glyph-atlas path. */
export class ViewLinesGpu extends Disposable {
	private readonly context: ViewGpuContext;
	private readonly vertexBuffer = this._register(new MutableDisposable<IReference<GPUBuffer>>());
	private readonly uniformBuffer = this._register(new MutableDisposable<IReference<GPUBuffer>>());
	private readonly atlasTexture = this._register(new MutableDisposable<IReference<GPUTexture>>());
	private readonly renderStrategy = this._register(new MutableDisposable<IGpuRenderStrategy>());
	private pipeline: GPURenderPipeline | undefined;
	private sampler: GPUSampler | undefined;
	private bindGroup: GPUBindGroup | undefined;
	private vertexBufferSize = 0;
	private atlasRevision = -1;
	private uploadedPageVersions: number[] = [];
	private rasterizer: GlyphRasterizer | undefined;
	private lastRenderingContext: EditorRenderingContext | undefined;
	private pendingRenderingContext: EditorRenderingContext | undefined;
	private rendering = false;

	constructor(private readonly options: ViewLinesGpuOptions) {
		super();
		this.context = this._register(new ViewGpuContext({ host: options.host }));
		this._register(this.context.onDidChange(() => {
			if (this.lastRenderingContext) this.render(this.lastRenderingContext);
		}));
	}

	public render(context: EditorRenderingContext): void {
		this.lastRenderingContext = context;
		this.pendingRenderingContext = context;
		if (this.rendering) return;
		this.rendering = true;
		try {
			while (this.pendingRenderingContext) {
				const next = this.pendingRenderingContext;
				this.pendingRenderingContext = undefined;
				this.renderFrame(next);
			}
		} catch (error) {
			this.showDomText();
			this.context.hideCanvas();
			this.context.markUnavailable(asError(error));
		} finally {
			this.rendering = false;
		}
	}

	private renderFrame(context: EditorRenderingContext): void {
		const layout = context.layout;
		this.context.layout(layout.viewportSize.width, layout.viewportSize.height, layout.scrollPosition.left, layout.scrollPosition.top);
		const overlay = context.overlay;
		if (layout.viewZones || this.context.status !== 'ready' || this.isForcedColors() || !overlay || !this.isRenderingContextCurrent(context, overlay.visualLineProjection)) {
			this.showDomText();
			this.context.hideCanvas();
			return;
		}
		const visualLines = overlay.visualLineProjection;
		this.validateRenderedLines(visualLines, this.options.viewLines.renderedLines);
		this.ensureResources(visualLines);
		const prepared = this.createFrame(context, visualLines);
		this.uploadAtlas();
		this.draw(prepared.frame, layout);
		this.applyRenderedLines(prepared.renderedLines, prepared.frame.gpuLineIndexes);
		this.context.showCanvas();
	}

	public invalidateFont(): void {
		this.rasterizer = undefined;
		this.renderStrategy.clear();
		this.context.clearAtlas();
	}

	public invalidateTokens(): void {
		this.renderStrategy.value?.reset();
	}

	private ensureResources(visualLines: EditorVisualLineProjection): void {
		const device = this.context.device;
		if (!this.rasterizer || this.rasterizer.devicePixelRatio !== this.context.devicePixelRatio) {
			this.rasterizer = new GlyphRasterizer(this.context.canvas.ownerDocument, this.context.devicePixelRatio);
			this.renderStrategy.value = this.createRenderStrategy(this.rasterizer, visualLines);
		}
		if (!this.pipeline) {
			const presentationFormat = this.context.canvas.ownerDocument.defaultView!.navigator.gpu.getPreferredCanvasFormat();
			const shader = device.createShaderModule({ label: 'Stanza ViewLinesGpu shader', code: this.renderStrategy.value!.wgsl });
			this.pipeline = device.createRenderPipeline({
				label: 'Stanza ViewLinesGpu pipeline',
				layout: 'auto',
				vertex: {
					module: shader,
					entryPoint: 'vertexMain',
					buffers: [{
						arrayStride: VERTEX_FLOAT_COUNT * Float32Array.BYTES_PER_ELEMENT,
						attributes: [
							{ shaderLocation: 0, offset: 0, format: 'float32x2' },
							{ shaderLocation: 1, offset: 2 * Float32Array.BYTES_PER_ELEMENT, format: 'float32x2' },
							{ shaderLocation: 2, offset: 4 * Float32Array.BYTES_PER_ELEMENT, format: 'float32' },
						],
					}],
				},
				fragment: {
					module: shader,
					entryPoint: 'fragmentMain',
					targets: [{
						format: presentationFormat,
						blend: {
							color: { srcFactor: 'src-alpha', dstFactor: 'one-minus-src-alpha', operation: 'add' },
							alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' },
						},
					}],
				},
				primitive: { topology: 'triangle-list' },
			});
			this.sampler = device.createSampler({ magFilter: 'nearest', minFilter: 'nearest' });
			this.uniformBuffer.value = GPULifecycle.createBuffer(device, {
				label: 'Stanza ViewLinesGpu dimensions',
				size: 6 * Float32Array.BYTES_PER_ELEMENT,
				usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
			});
		}
		if (this.atlasRevision !== this.context.textureAtlasRevision) this.createAtlasTexture();
	}

	private createAtlasTexture(): void {
		const atlas = this.context.atlas;
		this.renderStrategy.value?.reset();
		this.atlasTexture.value = GPULifecycle.createTexture(this.context.device, {
			label: 'Stanza ViewLinesGpu glyph atlas',
			size: { width: atlas.pageSize, height: atlas.pageSize, depthOrArrayLayers: atlas.pages.length },
			format: 'rgba8unorm',
			usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST | GPUTextureUsage.RENDER_ATTACHMENT,
		});
		this.atlasRevision = this.context.textureAtlasRevision;
		this.uploadedPageVersions = [];
		this.bindGroup = this.context.device.createBindGroup({
			label: 'Stanza ViewLinesGpu bindings',
			layout: this.pipeline!.getBindGroupLayout(0),
			entries: [
				{ binding: BindingId.Texture, resource: this.atlasTexture.value.object.createView({ dimension: '2d-array' }) },
				{ binding: BindingId.TextureSampler, resource: this.sampler! },
				{ binding: BindingId.LayoutInfoUniform, resource: { buffer: this.uniformBuffer.value!.object } },
				...this.renderStrategy.value!.bindGroupEntries,
			],
		});
	}

	private createFrame(context: EditorRenderingContext, visualLines: EditorVisualLineProjection): PreparedGpuFrame {
		const overlay = context.overlay;
		if (!overlay) throw new Error('WebGPU frame requires a version-bound overlay snapshot');
		const rootStyle = this.context.canvas.ownerDocument.defaultView!.getComputedStyle(this.options.host);
		this.ensureRenderStrategy(visualLines);
		const renderedLines = new Map(this.options.viewLines.renderedLines);
		const frame = this.renderStrategy.value!.update({
			layout: context.layout,
			model: this.options.model,
			visualLines,
			visibleLineIndexes: new Set(renderedLines.keys()),
			semanticTokenSource: this.options.semanticTokenSource,
			bracketColorizationSource: this.options.bracketColorizationSource,
			textLeft: overlay.textLeft,
			paddingTop: this.options.paddingTop,
			textDirection: this.options.viewLineOptions.textDirection,
			fontLigatures: this.options.viewLineOptions.fontLigatures,
			rootStyle,
			atlas: this.context.atlas,
		});
		return Object.freeze({ frame, renderedLines });
	}

	private uploadAtlas(): void {
		const atlas = this.context.atlas;
		if (this.atlasTexture.value && this.uploadedPageVersions.length !== atlas.pages.length) this.createAtlasTexture();
		for (const page of atlas.pages) {
			if (this.uploadedPageVersions[page.index] === page.version) continue;
			this.context.device.queue.copyExternalImageToTexture(
				{ source: page.source },
				{ texture: this.atlasTexture.value!.object, origin: { x: 0, y: 0, z: page.index } },
				{ width: page.usedArea.right, height: page.usedArea.bottom },
			);
			this.uploadedPageVersions[page.index] = page.version;
		}
	}

	private draw(frame: GpuRenderFrame, layout: EditorViewportLayout): void {
		const vertices = frame.vertices;
		const dimensions = this.context.devicePixelDimensions;
		const atlasSize = this.context.atlas.pageSize;
		this.context.device.queue.writeBuffer(this.uniformBuffer.value!.object, 0, new Float32Array([
			dimensions.width,
			dimensions.height,
			atlasSize,
			atlasSize,
			layout.scrollPosition.left * this.context.devicePixelRatio,
			layout.scrollPosition.top * this.context.devicePixelRatio,
		]));
		if (vertices.byteLength > this.vertexBufferSize) {
			this.vertexBufferSize = Math.max(vertices.byteLength, 4_096);
			this.vertexBuffer.value = GPULifecycle.createBuffer(this.context.device, {
				label: 'Stanza ViewLinesGpu vertices',
				size: this.vertexBufferSize,
				usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST,
			});
		}
		if (vertices.byteLength > 0) this.context.device.queue.writeBuffer(this.vertexBuffer.value!.object, 0, vertices as Float32Array<ArrayBuffer>);
		const encoder = this.context.device.createCommandEncoder({ label: 'Stanza ViewLinesGpu frame' });
		const textureView = this.context.context.getCurrentTexture().createView();
		this.context.rectangleRenderer.encode(encoder, textureView, dimensions.width, dimensions.height, layout.scrollPosition.left * this.context.devicePixelRatio, layout.scrollPosition.top * this.context.devicePixelRatio);
		const pass = encoder.beginRenderPass({
			label: 'Stanza ViewLinesGpu pass',
			colorAttachments: [{
				view: textureView,
				clearValue: { r: 0, g: 0, b: 0, a: 0 },
				loadOp: 'load',
				storeOp: 'store',
			}],
		});
		if (vertices.byteLength > 0) {
			pass.setPipeline(this.pipeline!);
			pass.setBindGroup(0, this.bindGroup!);
			pass.setVertexBuffer(0, this.vertexBuffer.value!.object);
			this.renderStrategy.value!.draw(pass, frame);
		}
		pass.end();
		this.context.device.queue.submit([encoder.finish()]);
	}

	private applyRenderedLines(renderedLines: ReadonlyMap<number, ViewLine>, gpuLineIndexes: ReadonlySet<number>): void {
		for (const [visualLineIndex, line] of renderedLines) line.domNode.domNode.classList.toggle('gpu-rendered', gpuLineIndexes.has(visualLineIndex));
	}

	private showDomText(): void {
		for (const line of this.options.viewLines.renderedLines.values()) line.domNode.domNode.classList.remove('gpu-rendered');
	}

	private isForcedColors(): boolean {
		const ownerWindow = this.context.canvas.ownerDocument.defaultView;
		return typeof ownerWindow?.matchMedia === 'function' && ownerWindow.matchMedia('(forced-colors: active)').matches;
	}

	private ensureRenderStrategy(visualLines: EditorVisualLineProjection): void {
		const expectedType = this.canUseFullFileStrategy(visualLines) ? 'fullfile' : 'viewport';
		if (this.renderStrategy.value?.type === expectedType) return;
		this.renderStrategy.value = this.createRenderStrategy(this.rasterizer!, visualLines);
	}

	private createRenderStrategy(rasterizer: GlyphRasterizer, visualLines: EditorVisualLineProjection): IGpuRenderStrategy {
		return this.canUseFullFileStrategy(visualLines)
			? new FullFileRenderStrategy(rasterizer)
			: new ViewportRenderStrategy(rasterizer);
	}

	private isRenderingContextCurrent(context: EditorRenderingContext, visualLines: EditorVisualLineProjection): boolean {
		return context.layout.modelVersion === this.options.model.version && visualLines.modelVersion === this.options.model.version;
	}

	private validateRenderedLines(visualLines: EditorVisualLineProjection, renderedLines: ReadonlyMap<number, ViewLine>): void {
		for (const visualLineIndex of renderedLines.keys()) {
			if (!visualLines.lineAt(visualLineIndex)) throw new Error('WebGPU rendered row is outside the visual-line projection');
		}
	}

	private canUseFullFileStrategy(visualLines: EditorVisualLineProjection): boolean {
		if (visualLines.visualLineCount > FullFileRenderStrategy.maxSupportedLines) return false;
		for (let lineIndex = 0; lineIndex < this.options.model.lineCount; lineIndex += 1) {
			if (this.options.model.getLineContent(lineIndex).length > FullFileRenderStrategy.maxSupportedColumns) return false;
		}
		return true;
	}
}

function asError(error: unknown): Error {
	return error instanceof Error ? error : new Error(String(error));
}
