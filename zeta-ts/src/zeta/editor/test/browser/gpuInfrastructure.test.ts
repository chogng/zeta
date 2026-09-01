import assert from 'node:assert/strict';
import test from 'node:test';
import { TextModel } from '../../common/model/textModel.js';
import { EditorVisualLineProjection } from '../../common/viewModel/modelLineProjection.js';
import { BufferDirtyTracker } from '../../browser/gpu/bufferDirtyTracker.js';
import { segmentContent } from '../../browser/gpu/contentSegmenter.js';
import { createObjectCollectionBuffer } from '../../browser/gpu/objectCollectionBuffer.js';
import { type TextureAtlas } from '../../browser/gpu/atlas/textureAtlas.js';
import { FullFileRenderStrategy } from '../../browser/gpu/renderStrategy/fullFileRenderStrategy.js';
import { type GlyphRasterizer } from '../../browser/gpu/raster/glyphRasterizer.js';
import { RectangleRenderer } from '../../browser/gpu/rectangleRenderer.js';

test('BufferDirtyTracker exposes one inclusive dirty range', () => {
	const tracker = new BufferDirtyTracker();
	assert.equal(tracker.isDirty, false);

	tracker.flag(8, 3);
	tracker.flag(2, 2);

	assert.deepEqual({ offset: tracker.dataOffset, size: tracker.dirtySize, dirty: tracker.isDirty }, { offset: 2, size: 9, dirty: true });
	tracker.clear();
	assert.deepEqual({ offset: tracker.dataOffset, size: tracker.dirtySize, dirty: tracker.isDirty }, { offset: undefined, size: undefined, dirty: false });
});

test('ObjectCollectionBuffer grows and compacts managed entries', () => {
	using collection = createObjectCollectionBuffer([{ name: 'x' }, { name: 'y' }] as const, 1);
	using first = collection.createEntry({ x: 1, y: 2 });
	using second = collection.createEntry({ x: 3, y: 4 });

	assert.equal(collection.entryCount, 2);
	assert.deepEqual([...collection.view.slice(0, collection.viewUsedSize)], [1, 2, 3, 4]);
	first.dispose();
	assert.equal(collection.entryCount, 1);
	assert.deepEqual([...collection.view.slice(0, collection.viewUsedSize)], [3, 4]);

	second.set('y', 9);
	assert.equal(second.get('y'), 9);
});

test('ContentSegmenter returns one entry for a complete grapheme', () => {
	const text = 'A👩‍💻B';
	const segmenter = segmentContent(text, false, false);

	assert.equal(segmenter.getSegmentAtIndex(0), 'A');
	assert.equal(segmenter.getSegmentAtIndex(1), '👩‍💻');
	assert.equal(segmenter.getSegmentAtIndex(2), undefined);
	assert.equal(segmenter.getSegmentAtIndex(text.length - 1), 'B');
});

test('Full-file GPU rendering starts at canonical coordinates and leaves subpixel placement to the atlas', () => {
	using model = new TextModel('abcd');
	const visualLines = EditorVisualLineProjection.fromBreakColumns(model, [[2, 4]], [16]);
	using strategy = new FullFileRenderStrategy({
		devicePixelRatio: 1,
		getTextMetrics: () => ({ width: 8 }),
	} as unknown as GlyphRasterizer);
	const frame = strategy.update({
		layout: {
			lineHeight: 20,
			viewportSize: { width: 200, height: 40 },
			contentSize: { width: 200, height: 40 },
			scrollPosition: { left: 0, top: 0 },
			maximumScrollPosition: { left: 0, top: 0 },
			visibleLines: { startLineIndex: 0, endLineIndexExclusive: 2 },
			renderLines: { startLineIndex: 0, endLineIndexExclusive: 2 },
			renderTop: 0,
		},
		model,
		visualLines,
		visibleLineIndexes: new Set([0, 1]),
		semanticTokenSource: undefined,
		bracketColorizationSource: undefined,
		textLeft: 44.2,
		paddingTop: 0,
		textDirection: 'ltr',
		fontLigatures: false,
		rootStyle: gpuRootStyle(),
		atlas: fixedGlyphAtlas(8.25),
	});

	assert.equal(frame.vertices[0], 44);
	assert.equal(frame.vertices[30], 52);
	assert.equal(frame.vertices[60], 60);
	assert.equal(frame.vertices[90], 68);
	assert.equal(frame.vertices.length, 4 * 6 * 5);
	assert.deepEqual(frame.lineGeometries.get(0), { leftByOffset: [44.2, 52.45, 60.7], newLineWidth: 8 });
	assert.deepEqual(frame.lineGeometries.get(1), { leftByOffset: [60.2, 68.45, 76.7], newLineWidth: 8 });
	const glyphBounds = Array.from({ length: 4 }, (_, glyphIndex) => {
		const yCoordinates = Array.from({ length: 6 }, (_, vertexIndex) => frame.vertices[glyphIndex * 30 + vertexIndex * 5 + 1]!);
		return {
			top: Math.min(...yCoordinates),
			bottom: Math.max(...yCoordinates),
		};
	});
	assert.deepEqual(glyphBounds.map(bounds => Math.floor((bounds.top + bounds.bottom) / 2 / 20)), [0, 0, 1, 1]);
	assert.ok(Math.max(glyphBounds[0]!.bottom, glyphBounds[1]!.bottom) < Math.min(glyphBounds[2]!.top, glyphBounds[3]!.top));
});

test('GPU frame geometry preserves tab stops and complete grapheme boundaries', () => {
	using model = new TextModel('a\t👩‍💻b');
	const visualLines = EditorVisualLineProjection.fromBreakColumns(model, [[model.getLineMaxColumn(1) - 1]]);
	using strategy = new FullFileRenderStrategy({
		devicePixelRatio: 1,
		getTextMetrics: () => ({ width: 8 }),
	} as unknown as GlyphRasterizer);
	const frame = strategy.update({
		layout: {
			lineHeight: 20,
			viewportSize: { width: 200, height: 20 },
			contentSize: { width: 200, height: 20 },
			scrollPosition: { left: 0, top: 0 },
			maximumScrollPosition: { left: 0, top: 0 },
			visibleLines: { startLineIndex: 0, endLineIndexExclusive: 1 },
			renderLines: { startLineIndex: 0, endLineIndexExclusive: 1 },
			renderTop: 0,
		},
		model,
		visualLines,
		visibleLineIndexes: new Set([0]),
		semanticTokenSource: undefined,
		bracketColorizationSource: undefined,
		textLeft: 10,
		paddingTop: 0,
		textDirection: 'ltr',
		fontLigatures: false,
		rootStyle: gpuRootStyle(),
		atlas: fixedGlyphAtlas(),
	});

	assert.deepEqual(frame.lineGeometries.get(0)?.leftByOffset, [10, 18, 42, 42, 42, 42, 42, 50, 58]);
});

test('Rectangle GPU rendering encodes a clear pass into the caller-owned frame', () => {
	const originalUsage = Object.getOwnPropertyDescriptor(globalThis, 'GPUBufferUsage');
	Object.defineProperty(globalThis, 'GPUBufferUsage', {
		configurable: true,
		value: { COPY_DST: 1, STORAGE: 2, UNIFORM: 4, VERTEX: 8 },
	});
	const passes: GPURenderPassDescriptor[] = [];
	let ended = false;
	const device = {
		queue: {
			writeBuffer: () => undefined,
		},
		createBuffer: () => ({ destroy: () => undefined }),
		createShaderModule: () => ({}),
		createRenderPipeline: () => ({ getBindGroupLayout: () => ({}) }),
		createBindGroup: () => ({}),
	} as unknown as GPUDevice;
	const encoder = {
			beginRenderPass: (descriptor: GPURenderPassDescriptor) => {
				passes.push(descriptor);
				return { end: () => { ended = true; } };
			},
		} as unknown as GPUCommandEncoder;
	const view = {} as GPUTextureView;

	try {
		using renderer = new RectangleRenderer(device, 'bgra8unorm');
		renderer.encode(encoder, view, 800, 600, 10, 20);
		const attachment = [...(passes[0]?.colorAttachments ?? [])][0] as GPURenderPassColorAttachment | undefined;
		assert.deepEqual({
			passCount: passes.length,
			view: attachment?.view,
			loadOp: attachment?.loadOp,
			storeOp: attachment?.storeOp,
			clearValue: attachment?.clearValue,
			ended,
		}, {
			passCount: 1,
			view,
			loadOp: 'clear',
			storeOp: 'store',
			clearValue: { r: 0, g: 0, b: 0, a: 0 },
			ended: true,
		});
	} finally {
		if (originalUsage) Object.defineProperty(globalThis, 'GPUBufferUsage', originalUsage);
		else Reflect.deleteProperty(globalThis, 'GPUBufferUsage');
	}
});

function gpuRootStyle(): CSSStyleDeclaration {
	return {
		color: '#222222',
		fontFamily: 'monospace',
		fontSize: '14px',
		fontStyle: 'normal',
		fontVariant: 'none',
		fontVariantCaps: 'normal',
		fontWeight: '400',
		letterSpacing: '0px',
		tabSize: '4',
		getPropertyValue: () => '',
	} as unknown as CSSStyleDeclaration;
}

function fixedGlyphAtlas(advance = 8): TextureAtlas {
	return {
		getGlyph: () => ({
			pageIndex: 0,
			glyphIndex: 0,
			x: 0,
			y: 0,
			w: 8,
			h: 10,
			originOffsetX: 0,
			originOffsetY: 0,
			advance,
			fontBoundingBoxAscent: 8,
			fontBoundingBoxDescent: 2,
		}),
	} as unknown as TextureAtlas;
}
