import assert from 'node:assert/strict';
import test from 'node:test';
import { type EditorViewportLayout } from '../../common/viewLayout/viewLayout.js';
import { type EditorViewportData } from '../../common/viewLayout/viewLinesViewportData.js';
import { createEditorRenderingContext, type EditorOverlayContext, type EditorRenderingContext } from '../../browser/view/renderingContext.js';

test('createEditorRenderingContext omits stale overlay geometry', () => {
	const layout = {} as EditorViewportLayout;
	const viewportData = { modelVersion: 4 } as EditorViewportData;
	const matchingOverlay = {
		model: { version: 4 },
		visualLineProjection: { modelVersion: 4 },
	} as unknown as EditorOverlayContext;
	const staleOverlay = {
		model: { version: 5 },
		visualLineProjection: { modelVersion: 4 },
	} as unknown as EditorOverlayContext;
	const staleViewportData = { modelVersion: 3 } as EditorViewportData;

	const current = createEditorRenderingContext(layout, matchingOverlay, viewportData);
	const stale = createEditorRenderingContext(layout, staleOverlay, viewportData);
	const staleViewportContext = createEditorRenderingContext(layout, matchingOverlay, staleViewportData);

	assert.equal(current.overlay, matchingOverlay);
	assert.equal(stale.overlay, undefined);
	assert.equal(staleViewportContext.overlay, undefined);
	assert.equal(current.viewportData, viewportData);
	assert.equal(Object.isFrozen(current), true);
});
