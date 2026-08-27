import './viewLinesGpu.css';
import { Disposable, MutableDisposable } from '../../../../base/common/lifecycle.js';
import { type EditorTextDirection } from '../../view.js';
import { type TextMeasurer } from '../../config/fontMeasurements.js';
import { ViewGpuContext } from '../../gpu/viewGpuContext.js';
import { GpuLifecycle, type GpuResourceReference } from '../../gpu/gpuDisposable.js';
import { type GpuGlyphStyle, GlyphRasterizer } from '../../gpu/raster/glyphRasterizer.js';
import { type EditorVisualLineProjection } from '../../../common/viewModel/modelLineProjection.js';
import { type EditorViewportLayout } from '../../../common/viewLayout/viewLayout.js';
import { type TextModel } from '../../../common/model/textModel.js';
import { SemanticTokenModifier, SemanticTokenPresentation, type BracketColorizationSource, type ResolvedSemanticToken, type SemanticTokenSource } from '../semanticTokens/semanticTokenPresentation.js';
import { type RenderedLine } from '../viewLines/renderedLine.js';
import { VIEW_LINES_GPU_SHADER } from './viewLinesGpu.wgsl.js';

export interface ViewLinesGpuOptions {
	readonly host: HTMLElement;
	readonly model: TextModel;
	readonly readVisualLines: () => EditorVisualLineProjection;
	readonly readRenderedLines: () => ReadonlyMap<number, RenderedLine>;
	readonly semanticTokenSource: SemanticTokenSource | undefined;
	readonly bracketColorizationSource: BracketColorizationSource | undefined;
	readonly textMeasurer: TextMeasurer;
	readonly readTextLeft: () => number;
	readonly paddingTop: number;
	readonly textDirection: EditorTextDirection;
	readonly fontLigatures: boolean;
	readonly onError: (error: Error) => void;
}

const MAXIMUM_GPU_COLUMN = 2_000;
const VERTEX_FLOAT_COUNT = 5;

/** Draws eligible visible text rows through the VS Code-aligned WebGPU glyph-atlas path. */
export class ViewLinesGpu extends Disposable {
	private readonly context: ViewGpuContext;
	private readonly vertexBuffer = this._register(new MutableDisposable<GpuResourceReference<GPUBuffer>>());
	private readonly uniformBuffer = this._register(new MutableDisposable<GpuResourceReference<GPUBuffer>>());
	private readonly atlasTexture = this._register(new MutableDisposable<GpuResourceReference<GPUTexture>>());
	private pipeline: GPURenderPipeline | undefined;
	private sampler: GPUSampler | undefined;
	private bindGroup: GPUBindGroup | undefined;
	private vertexBufferSize = 0;
	private atlasRevision = -1;
	private uploadedPageVersions: number[] = [];
	private rasterizer: GlyphRasterizer | undefined;
	private lastLayout: EditorViewportLayout | undefined;
	private rendering = false;

	constructor(private readonly options: ViewLinesGpuOptions) {
		super();
		this.context = this._register(new ViewGpuContext({ host: options.host, onError: options.onError }));
		this._register(this.context.onDidChange(() => {
			if (this.lastLayout) this.render(this.lastLayout);
		}));
	}

	public render(layout: EditorViewportLayout): void {
		this.lastLayout = layout;
		if (this.rendering) return;
		this.rendering = true;
		try {
			this.context.layout(layout.viewportSize.width, layout.viewportSize.height, layout.scrollPosition.left, layout.scrollPosition.top);
			if (this.context.status !== 'ready' || this.isForcedColors()) {
				this.showDomText();
				return;
			}
			this.ensureResources();
			const frame = this.createFrame(layout);
			this.uploadAtlas();
			this.draw(frame.vertices);
			this.applyRenderedLines(frame.gpuLineIndexes);
		} catch (error) {
			this.showDomText();
			this.context.markUnavailable(asError(error));
		} finally {
			this.rendering = false;
		}
	}

	public invalidateFont(): void {
		this.rasterizer = undefined;
		this.context.clearAtlas();
	}

	private ensureResources(): void {
		const device = this.context.device;
		if (!this.pipeline) {
			const shader = device.createShaderModule({ label: 'Stanza ViewLinesGpu shader', code: VIEW_LINES_GPU_SHADER });
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
						format: this.context.canvas.ownerDocument.defaultView!.navigator.gpu.getPreferredCanvasFormat(),
						blend: {
							color: { srcFactor: 'src-alpha', dstFactor: 'one-minus-src-alpha', operation: 'add' },
							alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' },
						},
					}],
				},
				primitive: { topology: 'triangle-list' },
			});
			this.sampler = device.createSampler({ magFilter: 'nearest', minFilter: 'nearest' });
			this.uniformBuffer.value = GpuLifecycle.createBuffer(device, {
				label: 'Stanza ViewLinesGpu dimensions',
				size: 4 * Float32Array.BYTES_PER_ELEMENT,
				usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
			});
		}
		if (this.atlasRevision !== this.context.textureAtlasRevision) this.createAtlasTexture();
		if (!this.rasterizer || this.rasterizer.devicePixelRatio !== this.context.devicePixelRatio) {
			this.rasterizer = new GlyphRasterizer(this.context.canvas.ownerDocument, this.context.devicePixelRatio);
		}
	}

	private createAtlasTexture(): void {
		const atlas = this.context.atlas;
		this.atlasTexture.value = GpuLifecycle.createTexture(this.context.device, {
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
				{ binding: 0, resource: this.atlasTexture.value.object.createView({ dimension: '2d-array' }) },
				{ binding: 1, resource: this.sampler! },
				{ binding: 2, resource: { buffer: this.uniformBuffer.value!.object } },
			],
		});
	}

	private createFrame(layout: EditorViewportLayout): { readonly vertices: Float32Array; readonly gpuLineIndexes: ReadonlySet<number> } {
		const rootStyle = this.context.canvas.ownerDocument.defaultView!.getComputedStyle(this.options.host);
		const visualLines = this.options.readVisualLines();
		const renderedLines = this.options.readRenderedLines();
		const devicePixelRatio = this.context.devicePixelRatio;
		const vertices: number[] = [];
		const gpuLineIndexes = new Set<number>();
		const baseStyle = readBaseStyle(rootStyle);
		const tabSize = positiveNumber(Number.parseFloat(rootStyle.tabSize), 4);
		for (const visualLineIndex of renderedLines.keys()) {
			const visualLine = visualLines.lineAt(visualLineIndex);
			if (!visualLine) continue;
			const text = this.options.model.getLineContent(visualLine.logicalLineIndex).slice(visualLine.startColumn, visualLine.endColumn);
			const tokens = this.options.semanticTokenSource?.getLineTokens(visualLine.logicalLineIndex) ?? [];
			const brackets = this.options.bracketColorizationSource?.getLineBrackets(visualLine.logicalLineIndex) ?? [];
			if (!this.canRenderLine(text, tokens)) continue;
			const lineStart = Math.ceil((this.options.readTextLeft() + this.options.textMeasurer.contentLeftPadding + (visualLine.wrappedTextIndentWidth ?? 0) - layout.scrollPosition.left) * devicePixelRatio);
			let deviceX = lineStart;
			const lineTop = (this.options.paddingTop + visualLineIndex * layout.lineHeight - layout.scrollPosition.top) * devicePixelRatio;
			const segments = segmentGraphemes(text);
			for (const segment of segments) {
				const logicalColumn = visualLine.startColumn + segment.index;
				const style = resolveGlyphStyle(baseStyle, rootStyle, tokens, brackets, logicalColumn);
				if (segment.segment === '\t') {
					const space = this.context.atlas.getGlyph(this.rasterizer!, ' ', style, deviceX);
					const tabStop = Math.max(1, space.advance * tabSize);
					deviceX = lineStart + (Math.floor((deviceX - lineStart) / tabStop) + 1) * tabStop;
					continue;
				}
				const glyph = this.context.atlas.getGlyph(this.rasterizer!, segment.segment, style, deviceX);
				const fontHeight = glyph.fontAscent + glyph.fontDescent;
				const baseline = lineTop + (layout.lineHeight * devicePixelRatio - fontHeight) / 2 + glyph.fontAscent;
				appendGlyphQuad(vertices, glyph, deviceX + glyph.offsetX, baseline + glyph.offsetY);
				deviceX += glyph.advance;
			}
			gpuLineIndexes.add(visualLineIndex);
		}
		return Object.freeze({ vertices: new Float32Array(vertices), gpuLineIndexes });
	}

	private canRenderLine(text: string, tokens: readonly ResolvedSemanticToken[]): boolean {
		if (this.options.fontLigatures || this.options.textDirection === 'rtl' || text.length > MAXIMUM_GPU_COLUMN || containsRtl(text)) return false;
		for (const token of tokens) {
			if (token.modifiers?.includes(SemanticTokenModifier.Static) || token.modifiers?.includes(SemanticTokenModifier.Deprecated)) return false;
			if (token.syntaxPresentation?.fontStyle?.some(style => style === 'underline' || style === 'strikethrough')) return false;
		}
		return true;
	}

	private uploadAtlas(): void {
		const atlas = this.context.atlas;
		if (this.atlasTexture.value && this.uploadedPageVersions.length !== atlas.pages.length) this.createAtlasTexture();
		for (const page of atlas.pages) {
			if (this.uploadedPageVersions[page.index] === page.version) continue;
			this.context.device.queue.copyExternalImageToTexture(
				{ source: page.source },
				{ texture: this.atlasTexture.value!.object, origin: { x: 0, y: 0, z: page.index } },
				{ width: page.usedWidth, height: page.usedHeight },
			);
			this.uploadedPageVersions[page.index] = page.version;
		}
	}

	private draw(vertices: Float32Array): void {
		const dimensions = this.context.devicePixelDimensions;
		const atlasSize = this.context.atlas.pageSize;
		this.context.device.queue.writeBuffer(this.uniformBuffer.value!.object, 0, new Float32Array([dimensions.width, dimensions.height, atlasSize, atlasSize]));
		if (vertices.byteLength > this.vertexBufferSize) {
			this.vertexBufferSize = Math.max(vertices.byteLength, 4_096);
			this.vertexBuffer.value = GpuLifecycle.createBuffer(this.context.device, {
				label: 'Stanza ViewLinesGpu vertices',
				size: this.vertexBufferSize,
				usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST,
			});
		}
		if (vertices.byteLength > 0) this.context.device.queue.writeBuffer(this.vertexBuffer.value!.object, 0, vertices as Float32Array<ArrayBuffer>);
		const encoder = this.context.device.createCommandEncoder({ label: 'Stanza ViewLinesGpu frame' });
		const pass = encoder.beginRenderPass({
			label: 'Stanza ViewLinesGpu pass',
			colorAttachments: [{
				view: this.context.context.getCurrentTexture().createView(),
				clearValue: { r: 0, g: 0, b: 0, a: 0 },
				loadOp: 'clear',
				storeOp: 'store',
			}],
		});
		if (vertices.byteLength > 0) {
			pass.setPipeline(this.pipeline!);
			pass.setBindGroup(0, this.bindGroup!);
			pass.setVertexBuffer(0, this.vertexBuffer.value!.object);
			pass.draw(vertices.length / VERTEX_FLOAT_COUNT);
		}
		pass.end();
		this.context.device.queue.submit([encoder.finish()]);
	}

	private applyRenderedLines(gpuLineIndexes: ReadonlySet<number>): void {
		for (const [visualLineIndex, line] of this.options.readRenderedLines()) line.domNode.domNode.classList.toggle('gpu-rendered', gpuLineIndexes.has(visualLineIndex));
	}

	private showDomText(): void {
		for (const line of this.options.readRenderedLines().values()) line.domNode.domNode.classList.remove('gpu-rendered');
	}

	private isForcedColors(): boolean {
		const ownerWindow = this.context.canvas.ownerDocument.defaultView;
		return typeof ownerWindow?.matchMedia === 'function' && ownerWindow.matchMedia('(forced-colors: active)').matches;
	}
}

function readBaseStyle(style: CSSStyleDeclaration): GpuGlyphStyle {
	return Object.freeze({
		color: style.color,
		fontFamily: style.fontFamily,
		fontSize: positiveNumber(Number.parseFloat(style.fontSize), 14),
		fontStyle: style.fontStyle || 'normal',
		fontVariant: style.fontVariant || 'normal',
		fontWeight: style.fontWeight || '400',
		letterSpacing: style.letterSpacing === 'normal' ? 0 : Number.parseFloat(style.letterSpacing) || 0,
	});
}

function resolveGlyphStyle(base: GpuGlyphStyle, rootStyle: CSSStyleDeclaration, tokens: readonly ResolvedSemanticToken[], brackets: readonly { readonly startColumn: number; readonly endColumn: number; readonly level: number }[], column: number): GpuGlyphStyle {
	const token = tokens.find(candidate => candidate.startColumn <= column && candidate.endColumn > column);
	const bracket = brackets.find(candidate => candidate.startColumn <= column && candidate.endColumn > column);
	const syntax = token?.syntaxPresentation;
	const tokenColor = token?.presentation ? cssVariable(rootStyle, tokenColorVariable(token.presentation), base.color) : base.color;
	const bracketColor = bracket ? cssVariable(rootStyle, bracketColorVariable(bracket.level), tokenColor) : tokenColor;
	const fontStyles = syntax?.fontStyle ?? [];
	return Object.freeze({
		...base,
		color: syntax?.foreground ?? bracketColor,
		fontStyle: fontStyles.includes('italic') || token?.modifiers?.some(modifier => modifier === SemanticTokenModifier.Readonly || modifier === SemanticTokenModifier.Abstract || modifier === SemanticTokenModifier.Async) ? 'italic' : base.fontStyle,
		fontWeight: fontStyles.includes('bold') ? 'bold' : token?.modifiers?.includes(SemanticTokenModifier.Declaration) ? '600' : base.fontWeight,
	});
}

function bracketColorVariable(level: number): string {
	switch ((level - 1) % 6 + 1) {
		case 1: return '--zeta-editor-token-keyword-foreground';
		case 2: return '--zeta-editor-token-function-foreground';
		case 3: return '--zeta-editor-token-type-foreground';
		case 4: return '--zeta-editor-token-number-foreground';
		case 5: return '--zeta-editor-token-string-foreground';
		case 6: return '--zeta-editor-token-variable-foreground';
	}
	throw new RangeError('WebGPU bracket level must be a positive integer');
}

function tokenColorVariable(presentation: SemanticTokenPresentation): string {
	switch (presentation) {
		case SemanticTokenPresentation.Comment: return '--zeta-editor-token-comment-foreground';
		case SemanticTokenPresentation.Keyword: return '--zeta-editor-token-keyword-foreground';
		case SemanticTokenPresentation.String: return '--zeta-editor-token-string-foreground';
		case SemanticTokenPresentation.Number: return '--zeta-editor-token-number-foreground';
		case SemanticTokenPresentation.Regexp: return '--zeta-editor-token-regexp-foreground';
		case SemanticTokenPresentation.Type: return '--zeta-editor-token-type-foreground';
		case SemanticTokenPresentation.Function: return '--zeta-editor-token-function-foreground';
		case SemanticTokenPresentation.Variable: return '--zeta-editor-token-variable-foreground';
		case SemanticTokenPresentation.Operator: return '--zeta-editor-token-operator-foreground';
	}
}

function cssVariable(style: CSSStyleDeclaration, name: string, defaultValue: string): string {
	return style.getPropertyValue(name).trim() || defaultValue;
}

function segmentGraphemes(text: string): readonly Intl.SegmentData[] {
	return [...new Intl.Segmenter(undefined, { granularity: 'grapheme' }).segment(text)];
}

function appendGlyphQuad(vertices: number[], glyph: { readonly pageIndex: number; readonly x: number; readonly y: number; readonly width: number; readonly height: number }, left: number, top: number): void {
	const right = left + glyph.width;
	const bottom = top + glyph.height;
	const atlasRight = glyph.x + glyph.width;
	const atlasBottom = glyph.y + glyph.height;
	vertices.push(
		left, top, glyph.x, glyph.y, glyph.pageIndex,
		right, top, atlasRight, glyph.y, glyph.pageIndex,
		left, bottom, glyph.x, atlasBottom, glyph.pageIndex,
		left, bottom, glyph.x, atlasBottom, glyph.pageIndex,
		right, top, atlasRight, glyph.y, glyph.pageIndex,
		right, bottom, atlasRight, atlasBottom, glyph.pageIndex,
	);
}

function containsRtl(text: string): boolean {
	return /[\u0590-\u08ff\ufb1d-\ufefc]/u.test(text);
}

function positiveNumber(value: number, defaultValue: number): number {
	return Number.isFinite(value) && value > 0 ? value : defaultValue;
}

function asError(error: unknown): Error {
	return error instanceof Error ? error : new Error(String(error));
}
