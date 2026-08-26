import assert from 'node:assert/strict';
import test from 'node:test';
import { type EditorViewportLayout } from '../../common/viewLayout/editorViewportModel.js';
import { type ViewportOverlayContext } from '../../browser/viewparts/viewportOverlay/viewportOverlayPresentation.js';
import { createEditorRenderingContext, type EditorRenderingContext } from '../../browser/view/renderingContext.js';
import { EditorViewContext, EditorViewPart, EditorViewPartCollection } from '../../browser/view/viewPart.js';

test('EditorViewPartCollection prepares every part before rendering with one context', () => {
	const layout = {} as EditorViewportLayout;
	const renderingContext = { layout, overlay: undefined } as EditorRenderingContext;
	const phases: string[] = [];
	const received: EditorRenderingContext[] = [];
	using parts = new EditorViewPartCollection();

	parts.register(new RecordingPart('first', phases, received));
	parts.register(new RecordingPart('second', phases, received));

	parts.prepareRender(renderingContext);
	parts.render(renderingContext);

	assert.deepEqual(phases, ['prepare:first', 'prepare:second', 'render:first', 'render:second']);
	assert.equal(received.length, 4);
	for (const context of received) {
		assert.equal(context, renderingContext);
	}
});

class RecordingPart extends EditorViewPart {
	constructor(
		private readonly name: string,
		private readonly phases: string[],
		private readonly received: EditorRenderingContext[],
	) {
		super();
	}

	public override prepareRender(context: EditorRenderingContext): void {
		this.phases.push(`prepare:${this.name}`);
		this.received.push(context);
	}

	public render(context: EditorRenderingContext): void {
		this.phases.push(`render:${this.name}`);
		this.received.push(context);
	}
}

test('EditorViewContext creates a rendering context from the current layout', () => {
	const layout = {} as EditorViewportLayout;
	const renderingContext = { layout, overlay: undefined } as EditorRenderingContext;
	let providedLayout: EditorViewportLayout | undefined;
	const context = new EditorViewContext(
		() => layout,
		provided => {
			providedLayout = provided;
			return renderingContext;
		},
	);

	assert.equal(context.renderingContext, renderingContext);
	assert.equal(providedLayout, layout);
});

test('createEditorRenderingContext omits stale overlay geometry', () => {
	const layout = {} as EditorViewportLayout;
	const matchingOverlay = {
		model: { version: 4 },
		visualLineProjection: { modelVersion: 4 },
	} as unknown as ViewportOverlayContext;
	const staleOverlay = {
		model: { version: 5 },
		visualLineProjection: { modelVersion: 4 },
	} as unknown as ViewportOverlayContext;

	const current = createEditorRenderingContext(layout, matchingOverlay);
	const stale = createEditorRenderingContext(layout, staleOverlay);

	assert.equal(current.overlay, matchingOverlay);
	assert.equal(stale.overlay, undefined);
	assert.equal(Object.isFrozen(current), true);
});
