import assert from 'node:assert/strict';
import test from 'node:test';
import { TextModel } from '../../common/model/textModel.js';
import { EditorVisualLineProjection } from '../../common/viewModel/modelLineProjection.js';
import { type EditorTextDirection } from '../../browser/view.js';
import { BufferDirtyTracker } from '../../browser/gpu/bufferDirtyTracker.js';
import { createContentSegmenter } from '../../browser/gpu/contentSegmenter.js';
import { createObjectCollectionBuffer } from '../../browser/gpu/objectCollectionBuffer.js';
import { type TextureAtlas } from '../../browser/gpu/atlas/textureAtlas.js';
import { FullFileRenderStrategy } from '../../browser/gpu/renderStrategy/fullFileRenderStrategy.js';
import { type GlyphRasterizer } from '../../browser/gpu/raster/glyphRasterizer.js';

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
	const segmenter = createContentSegmenter(text, { isBasicASCII: false, useMonospaceOptimizations: false });

	assert.equal(segmenter.getSegmentAtIndex(0), 'A');
	assert.equal(segmenter.getSegmentAtIndex(1), '👩‍💻');
	assert.equal(segmenter.getSegmentAtIndex(2), undefined);
	assert.equal(segmenter.getSegmentAtIndex(text.length - 1), 'B');
});

test('Full-file GPU rendering starts text at the canonical content coordinate', () => {
	using model = new TextModel('abcd');
	const visualLines = EditorVisualLineProjection.fromBreakColumns(model, [[2, 4]], [16]);
	using strategy = new FullFileRenderStrategy({ devicePixelRatio: 1 } as unknown as GlyphRasterizer);
	const frame = strategy.update({
		layout: {
			modelVersion: model.version,
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
		textLeft: 44,
		paddingTop: 0,
		textDirection: 'ltr' as EditorTextDirection,
		fontLigatures: false,
		rootStyle: gpuRootStyle(),
		atlas: fixedGlyphAtlas(),
	});

	assert.equal(frame.vertices[0], 44);
	assert.equal(frame.vertices[60], 60);
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

function fixedGlyphAtlas(): TextureAtlas {
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
			advance: 8,
			fontBoundingBoxAscent: 8,
			fontBoundingBoxDescent: 2,
		}),
	} as unknown as TextureAtlas;
}
