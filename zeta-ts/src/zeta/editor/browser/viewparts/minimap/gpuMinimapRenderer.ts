import { type MinimapRow } from "./minimapProjection.js";
import { MINIMAP_CONTENT_RIGHT, MINIMAP_LINE_HEIGHT, minimapContentWidth } from "./minimapPresentation.js";

const VERTEX_SHADER_SOURCE = `
  attribute vec2 position;
  void main() {
    gl_Position = vec4(position, 0.0, 1.0);
  }
`;

const FRAGMENT_SHADER_SOURCE = `
  precision mediump float;
  uniform vec4 color;
  void main() {
    gl_FragColor = color;
  }
`;

/**
 * GPU-backed density projection for Stanza's minimap.
 *
 * Text, selection and accessibility remain DOM-owned. This renderer owns only
 * the bounded, non-semantic minimap density rectangles and is optional: callers
 * must retain a DOM fallback when WebGL is unavailable or loses its context.
 */
export class GpuMinimapRenderer {
	private readonly program: WebGLProgram;
	private readonly positionLocation: number;
	private readonly colorLocation: WebGLUniformLocation;
	private readonly vertexBuffer: WebGLBuffer;
	private rows: readonly MinimapRow[] = [];
	private lineCount = 1;
	private width = 0;
	private height = 0;
	private available = true;

	private constructor(
		private readonly canvas: HTMLCanvasElement,
		private readonly context: WebGLRenderingContext,
	) {
		const program = createProgram(context, VERTEX_SHADER_SOURCE, FRAGMENT_SHADER_SOURCE);
		const positionLocation = context.getAttribLocation(program, "position");
		const colorLocation = context.getUniformLocation(program, "color");
		const vertexBuffer = context.createBuffer();
		if (positionLocation < 0 || !colorLocation || !vertexBuffer) {
			context.deleteProgram(program);
			throw new Error("Stanza GPU minimap could not initialize its rendering resources");
		}
		this.program = program;
		this.positionLocation = positionLocation;
		this.colorLocation = colorLocation;
		this.vertexBuffer = vertexBuffer;
	}

	/** Returns a renderer only when the current browser can provide WebGL. */
	static tryCreate(canvas: HTMLCanvasElement): GpuMinimapRenderer | undefined {
		if (canvas.ownerDocument.defaultView?.navigator.userAgent.includes("jsdom")) {
			return undefined;
		}
		try {
			const context = canvas.getContext("webgl", {
				alpha: true,
				antialias: false,
				depth: false,
				preserveDrawingBuffer: false,
				stencil: false,
			});
			return context ? new GpuMinimapRenderer(canvas, context) : undefined;
		} catch {
			return undefined;
		}
	}

	/** Whether this renderer can still issue WebGL commands for its canvas. */
	get isAvailable(): boolean {
		return this.available;
	}

	/** Stops GPU rendering after a context loss so the caller can restore its DOM fallback. */
	disable(): void {
		this.available = false;
	}

	/** Replaces the current bounded density data and redraws the current surface. */
	setRows(rows: readonly MinimapRow[], lineCount: number): void {
		if (!Number.isSafeInteger(lineCount) || lineCount < 1) {
			throw new RangeError("Stanza GPU minimap line count must be a positive safe integer");
		}
		this.rows = rows;
		this.lineCount = lineCount;
		this.draw();
	}

	/** Resizes in CSS pixels; backing storage follows the current device pixel ratio. */
	resize(width: number, height: number): void {
		if (!Number.isFinite(width) || width < 0 || !Number.isFinite(height) || height < 0) {
			throw new RangeError("Stanza GPU minimap size must be non-negative and finite");
		}
		if (this.width === width && this.height === height) return;
		this.width = width;
		this.height = height;
		this.draw();
	}

	dispose(): void {
		this.context.deleteBuffer(this.vertexBuffer);
		this.context.deleteProgram(this.program);
	}

	private draw(): void {
		if (!this.available) return;
		const pixelRatio = devicePixelRatioFor(this.canvas);
		const pixelWidth = Math.max(1, Math.ceil(this.width * pixelRatio));
		const pixelHeight = Math.max(1, Math.ceil(this.height * pixelRatio));
		if (this.canvas.width !== pixelWidth) this.canvas.width = pixelWidth;
		if (this.canvas.height !== pixelHeight) this.canvas.height = pixelHeight;
		this.context.viewport(0, 0, pixelWidth, pixelHeight);
		this.context.clearColor(0, 0, 0, 0);
		this.context.clear(this.context.COLOR_BUFFER_BIT);
		if (this.rows.length === 0 || this.width <= 0 || this.height <= 0) return;

		this.context.useProgram(this.program);
		this.context.bindBuffer(this.context.ARRAY_BUFFER, this.vertexBuffer);
		this.context.bufferData(this.context.ARRAY_BUFFER, minimapVertices(this.rows, this.lineCount, this.width, this.height), this.context.DYNAMIC_DRAW);
		this.context.enableVertexAttribArray(this.positionLocation);
		this.context.vertexAttribPointer(this.positionLocation, 2, this.context.FLOAT, false, 0, 0);
		this.context.uniform4fv(this.colorLocation, minimapForegroundColor(this.canvas));
		this.context.drawArrays(this.context.TRIANGLES, 0, this.rows.length * 6);
	}
}

function createProgram(context: WebGLRenderingContext, vertexSource: string, fragmentSource: string): WebGLProgram {
	const vertexShader = compileShader(context, context.VERTEX_SHADER, vertexSource);
	const fragmentShader = compileShader(context, context.FRAGMENT_SHADER, fragmentSource);
	const program = context.createProgram();
	if (!program) throw new Error("Stanza GPU minimap could not create a WebGL program");
	context.attachShader(program, vertexShader);
	context.attachShader(program, fragmentShader);
	context.linkProgram(program);
	context.deleteShader(vertexShader);
	context.deleteShader(fragmentShader);
	if (!context.getProgramParameter(program, context.LINK_STATUS)) {
		context.deleteProgram(program);
		throw new Error("Stanza GPU minimap could not link its WebGL program");
	}
	return program;
}

function compileShader(context: WebGLRenderingContext, kind: number, source: string): WebGLShader {
	const shader = context.createShader(kind);
	if (!shader) throw new Error("Stanza GPU minimap could not create a WebGL shader");
	context.shaderSource(shader, source);
	context.compileShader(shader);
	if (context.getShaderParameter(shader, context.COMPILE_STATUS)) return shader;
	context.deleteShader(shader);
	throw new Error("Stanza GPU minimap could not compile a WebGL shader");
}

function minimapVertices(rows: readonly MinimapRow[], lineCount: number, width: number, height: number): Float32Array {
	const vertices = new Float32Array(rows.length * 12);
	for (let index = 0; index < rows.length; index += 1) {
		const row = rows[index]!;
		const right = 1 - 2 * MINIMAP_CONTENT_RIGHT / width;
		const left = right - 2 * minimapContentWidth(row.density, width) / width;
		const top = 1 - 2 * row.startLineIndex / lineCount;
		const bottom = Math.max(-1, top - 2 * MINIMAP_LINE_HEIGHT / height);
		vertices.set([left, top, right, top, left, bottom, left, bottom, right, top, right, bottom], index * 12);
	}
	return vertices;
}

function minimapForegroundColor(canvas: HTMLCanvasElement): Float32Array {
	const value = canvas.ownerDocument.defaultView?.getComputedStyle(canvas).color;
	const match = value?.match(/^rgba?\((\d+),\s*(\d+),\s*(\d+)(?:,\s*([\d.]+))?\)$/);
	if (!match) return new Float32Array([0.65, 0.65, 0.65, 0.42]);
	return new Float32Array([
		Number(match[1]) / 255,
		Number(match[2]) / 255,
		Number(match[3]) / 255,
		(match[4] === undefined ? 1 : Number(match[4])) * 0.42,
	]);
}

function devicePixelRatioFor(canvas: HTMLCanvasElement): number {
	const value = canvas.ownerDocument.defaultView?.devicePixelRatio ?? 1;
	return Number.isFinite(value) && value > 0 ? value : 1;
}
