import assert from 'node:assert/strict';
import test from 'node:test';
import { MinimapRenderLayout } from '../../browser/viewParts/minimap/minimap.js';
import { RenderMinimap, type EditorMinimapLayoutInfo } from '../../common/config/editorOptions.js';
import type { EditorViewportLayout } from '../../common/viewLayout/viewLayout.js';

test('proportional minimap follows a long document from its first to its final visual rows', () => {
	const top = MinimapRenderLayout.create({
		editorLayout: createEditorLayout(0),
		minimapLayout: createMinimapLayout(),
		visualLineCount: 1_000,
		paddingTop: 0,
		paddingBottom: 0,
	});
	const bottom = MinimapRenderLayout.create({
		editorLayout: createEditorLayout(19_400),
		minimapLayout: createMinimapLayout(),
		visualLineCount: 1_000,
		paddingTop: 0,
		paddingBottom: 0,
	});

	assert.deepEqual(readLayoutGeometry(top), {
		startVisualLineIndex: 0,
		endVisualLineIndexExclusive: 300,
		sliderNeeded: true,
		sliderTop: 0,
		sliderHeight: 60,
	});
	assert.deepEqual(readLayoutGeometry(bottom), {
		startVisualLineIndex: 700,
		endVisualLineIndexExclusive: 1_000,
		sliderNeeded: true,
		sliderTop: 540,
		sliderHeight: 60,
	});
	assert.equal(bottom.lineSpan(0, 1), undefined);
	assert.deepEqual(bottom.lineSpan(999, 1_000), { top: 598, height: 2 });
	assert.equal(bottom.scrollTopAt(600), 19_400);
	assert.equal(bottom.scrollTopAtSliderPosition(600, 30), 19_400);
});

test('proportional minimap keeps padding and wrapped marker rows in canvas coordinates', () => {
	const layout = MinimapRenderLayout.create({
		editorLayout: createEditorLayout(0, 18, 400),
		minimapLayout: createMinimapLayout(),
		visualLineCount: 18,
		paddingTop: 20,
		paddingBottom: 0,
	});

	assert.deepEqual(readLayoutGeometry(layout), {
		startVisualLineIndex: 0,
		endVisualLineIndexExclusive: 18,
		sliderNeeded: false,
		sliderTop: 0,
		sliderHeight: 60,
	});
	assert.equal(layout.topPaddingInnerHeight, 2);
	assert.deepEqual(layout.lineSpan(12, 14), { top: 26, height: 4 });
});

test('sampling minimap maps markers and pointer navigation across the full document', () => {
	const layout = MinimapRenderLayout.create({
		editorLayout: createEditorLayout(9_700),
		minimapLayout: createMinimapLayout({
			minimapHeightIsEditorHeight: true,
			minimapIsSampling: true,
			minimapLineHeight: 1,
		}),
		visualLineCount: 1_000,
		paddingTop: 0,
		paddingBottom: 0,
	});

	assert.deepEqual(readLayoutGeometry(layout), {
		startVisualLineIndex: 0,
		endVisualLineIndexExclusive: 1_000,
		sliderNeeded: true,
		sliderTop: 291,
		sliderHeight: 18,
	});
	assert.deepEqual(readLineSpan(layout, 900, 901), { top: 540, height: 0.6 });
	assert.equal(layout.scrollTopAt(600), 19_400);
});

test('proportional minimap uses physical canvas capacity at high pixel ratios', () => {
	const layout = MinimapRenderLayout.create({
		editorLayout: createEditorLayout(19_400),
		minimapLayout: createMinimapLayout({
			minimapCanvasInnerWidth: 240,
			minimapCanvasInnerHeight: 1_200,
		}),
		visualLineCount: 1_000,
		paddingTop: 0,
		paddingBottom: 0,
	});

	assert.equal(layout.startVisualLineIndex, 400);
	assert.equal(layout.endVisualLineIndexExclusive, 1_000);
	assert.deepEqual(layout.lineSpan(999, 1_000), { top: 599, height: 1 });
});

function createEditorLayout(scrollTop: number, visualLineCount = 1_000, contentHeight = visualLineCount * 20): EditorViewportLayout {
	const maximumScrollTop = Math.max(0, contentHeight - 600);
	const visibleStart = Math.min(visualLineCount, Math.floor(scrollTop / 20));
	return Object.freeze({
		modelVersion: 1,
		lineHeight: 20,
		viewportSize: Object.freeze({ width: 800, height: 600 }),
		contentSize: Object.freeze({ width: 800, height: Math.max(600, contentHeight) }),
		scrollPosition: Object.freeze({ left: 0, top: scrollTop }),
		maximumScrollPosition: Object.freeze({ left: 0, top: maximumScrollTop }),
		visibleLines: Object.freeze({ startLineIndex: visibleStart, endLineIndexExclusive: Math.min(visualLineCount, visibleStart + 30) }),
		renderLines: Object.freeze({ startLineIndex: visibleStart, endLineIndexExclusive: Math.min(visualLineCount, visibleStart + 32) }),
		renderTop: visibleStart * 20,
	});
}

function createMinimapLayout(overrides: Partial<EditorMinimapLayoutInfo> = {}): EditorMinimapLayoutInfo {
	return Object.freeze({
		renderMinimap: RenderMinimap.Text,
		minimapLeft: 680,
		minimapWidth: 106,
		minimapHeightIsEditorHeight: false,
		minimapIsSampling: false,
		minimapScale: 1,
		minimapLineHeight: 2,
		minimapCanvasInnerWidth: 120,
		minimapCanvasInnerHeight: 600,
		minimapCanvasOuterWidth: 120,
		minimapCanvasOuterHeight: 600,
		...overrides,
	});
}

function readLayoutGeometry(layout: MinimapRenderLayout): object {
	return {
		startVisualLineIndex: layout.startVisualLineIndex,
		endVisualLineIndexExclusive: layout.endVisualLineIndexExclusive,
		sliderNeeded: layout.sliderNeeded,
		sliderTop: layout.sliderTop,
		sliderHeight: layout.sliderHeight,
	};
}

function readLineSpan(layout: MinimapRenderLayout, startVisualLineIndex: number, endVisualLineIndexExclusive: number): object | undefined {
	const span = layout.lineSpan(startVisualLineIndex, endVisualLineIndexExclusive);
	return span && {
		top: Number(span.top.toFixed(3)),
		height: Number(span.height.toFixed(3)),
	};
}
