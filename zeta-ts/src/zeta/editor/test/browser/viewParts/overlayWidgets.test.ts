import assert from 'node:assert/strict';
import test from 'node:test';
import { JSDOM } from 'jsdom';
import { Emitter } from '../../../../base/common/event.js';
import { OverlayWidgetPositionPreference, type IOverlayWidget, type IOverlayWidgetPosition } from '../../../browser/editorBrowser.js';
import { ViewOverlayWidgets } from '../../../browser/viewParts/overlayWidgets/overlayWidgets.js';
import { type ViewContext } from '../../../common/viewModel/viewContext.js';

test('ViewOverlayWidgets reports only position changes and owns widget DOM lifetime', () => {
	const dom = new JSDOM('<!doctype html><body><main></main></body>');
	const host = dom.window.document.querySelector<HTMLElement>('main')!;
	const minimumWidths: number[] = [];
	let rendered = 0;
	let minimumWidth = 40;
	const layout = new Emitter<void>();
	const initial: IOverlayWidgetPosition = { preference: OverlayWidgetPositionPreference.TOP_RIGHT_CORNER };
	const node = dom.window.document.createElement('section');
	const widget: IOverlayWidget = {
		onDidLayout: layout.event,
		getId: () => 'test.overlay',
		getDomNode: () => node,
		getPosition: () => initial,
		getMinContentWidthInPx: () => minimumWidth,
	};
	const overlays = new ViewOverlayWidgets(testViewContext(), {
		viewDomNode: host,
		allowOverflow: true,
		fixedOverflowWidgets: false,
		verticalScrollbarWidth: 12,
		horizontalScrollbarHeight: 10,
		readMinimapWidth: () => 0,
		setMinimumContentWidth: width => minimumWidths.push(width),
		requestRender: () => rendered += 1,
	});

	overlays.addWidget(widget);
	assert.equal(node.parentElement, overlays.getDomNode().domNode);
	assert.equal(overlays.setWidgetPosition(widget, initial), false);
	assert.equal(rendered, 0);

	minimumWidth = 64;
	assert.equal(overlays.setWidgetPosition(widget, { preference: OverlayWidgetPositionPreference.TOP_RIGHT_CORNER }), false);
	assert.equal(minimumWidths.at(-1), 64);
	assert.equal(overlays.setWidgetPosition(widget, { preference: { top: 8, left: 13 }, stackOrdinal: 2 }), true);
	assert.equal(overlays.setWidgetPosition(widget, { preference: { top: 8, left: 13 }, stackOrdinal: 2 }), false);

	layout.fire();
	assert.equal(rendered, 1);
	overlays.removeWidget(widget);
	assert.equal(node.isConnected, false);
	assert.equal(minimumWidths.at(-1), 0);
	overlays.dispose();
	layout.dispose();
	dom.window.close();
});

function testViewContext(): ViewContext {
	return { addEventHandler() {}, removeEventHandler() {} } as unknown as ViewContext;
}
