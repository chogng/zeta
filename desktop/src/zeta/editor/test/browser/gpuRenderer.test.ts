import assert from "node:assert/strict";
import test from "node:test";
import { GpuMinimapRenderer } from "../../browser/viewparts/minimap/gpuMinimapRenderer.js";

test("GPU minimap renders bounded density rows at device resolution and can yield to a fallback", () => {
  const calls: string[] = [];
  let vertices: Float32Array | undefined;
  let foreground: Float32Array | undefined;
  const context = fakeWebGlContext(calls, value => { vertices = value; }, value => { foreground = value; });
  const canvas = fakeCanvas(context);
  const renderer = GpuMinimapRenderer.tryCreate(canvas);
  assert.ok(renderer);

  renderer.resize(56, 100);
  renderer.setRows([
    { startLineIndex: 0, endLineIndexExclusive: 1, density: 0.5 },
    { startLineIndex: 50, endLineIndexExclusive: 55, density: 1 },
  ], 100);

  assert.equal(canvas.width, 112);
  assert.equal(canvas.height, 200);
  assert.deepEqual(calls.filter(call => call.startsWith("draw:")), ["draw:12"]);
  assert.ok(vertices);
  assertClose((1 - vertices[2]!) * 56 / 2, 4);
  assertClose((vertices[2]! - vertices[0]!) * 56 / 2, 22);
  assertClose((vertices[1]! - vertices[5]!) * 100 / 2, 1);
  assert.ok(foreground);
  assertClose(foreground[3]!, 0.42);
  assert.equal(renderer.isAvailable, true);

  renderer.disable();
  renderer.setRows([], 100);
  assert.equal(renderer.isAvailable, false);
  assert.deepEqual(calls.filter(call => call.startsWith("draw:")), ["draw:12"]);
  renderer.dispose();
  assert.ok(calls.includes("delete-buffer"));
  assert.ok(calls.includes("delete-program"));
});

test("GPU minimap declines test and non-WebGL canvases without affecting its DOM caller", () => {
  const canvas = fakeCanvas(undefined, "jsdom");
  assert.equal(GpuMinimapRenderer.tryCreate(canvas), undefined);
});

function fakeCanvas(context: WebGLRenderingContext | undefined, userAgent = "Electron"): HTMLCanvasElement {
  return {
    width: 0,
    height: 0,
    ownerDocument: {
      defaultView: {
        devicePixelRatio: 2,
        navigator: { userAgent },
        getComputedStyle: () => ({ color: "rgb(128, 64, 32)" }),
      },
    },
    getContext: () => context ?? null,
  } as unknown as HTMLCanvasElement;
}

function fakeWebGlContext(calls: string[], captureVertices?: (value: Float32Array) => void, captureForeground?: (value: Float32Array) => void): WebGLRenderingContext {
  const shader = {} as WebGLShader;
  const program = {} as WebGLProgram;
  const buffer = {} as WebGLBuffer;
  const uniform = {} as WebGLUniformLocation;
  return {
    ARRAY_BUFFER: 0x8892,
    COLOR_BUFFER_BIT: 0x4000,
    COMPILE_STATUS: 0x8b81,
    DYNAMIC_DRAW: 0x88e8,
    FLOAT: 0x1406,
    FRAGMENT_SHADER: 0x8b30,
    LINK_STATUS: 0x8b82,
    STATIC_DRAW: 0x88e4,
    TRIANGLES: 0x0004,
    VERTEX_SHADER: 0x8b31,
    attachShader: () => undefined,
    bindBuffer: () => undefined,
    bufferData: (_target: number, value: unknown) => {
      if (value instanceof Float32Array) captureVertices?.(value);
    },
    clear: () => undefined,
    clearColor: () => undefined,
    compileShader: () => undefined,
    createBuffer: () => buffer,
    createProgram: () => program,
    createShader: () => shader,
    deleteBuffer: () => calls.push("delete-buffer"),
    deleteProgram: () => calls.push("delete-program"),
    deleteShader: () => undefined,
    drawArrays: (_mode: number, _first: number, count: number) => calls.push(`draw:${count}`),
    enableVertexAttribArray: () => undefined,
    getAttribLocation: () => 0,
    getProgramParameter: () => true,
    getShaderParameter: () => true,
    getUniformLocation: () => uniform,
    linkProgram: () => undefined,
    shaderSource: () => undefined,
    uniform4fv: (_location: WebGLUniformLocation | null, value: Float32List) => captureForeground?.(new Float32Array(value)),
    useProgram: () => undefined,
    vertexAttribPointer: () => undefined,
    viewport: () => undefined,
  } as unknown as WebGLRenderingContext;
}

function assertClose(actual: number, expected: number): void {
  assert.equal(Math.abs(actual - expected) < 0.000_01, true);
}
