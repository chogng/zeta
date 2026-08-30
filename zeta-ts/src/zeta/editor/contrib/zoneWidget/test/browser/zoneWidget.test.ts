import assert from 'node:assert/strict';
import test from 'node:test';
import { JSDOM } from 'jsdom';
import { h } from '../../../../../base/browser/dom.js';
import { type TextMeasurer } from '../../../../common/viewModel/textMeasurer.js';
import { Position } from '../../../../common/core/position.js';
import { Range } from '../../../../common/core/range.js';
import { TextModel } from '../../../../common/model/textModel.js';
import { type ZoneWidgetEditor, type ZoneWidgetOptions } from '../../browser/zoneWidget.js';

const browserEnvironment = new JSDOM('<!doctype html><body></body>');
for (const [name, value] of Object.entries({
	window: browserEnvironment.window,
	document: browserEnvironment.window.document,
	Node: browserEnvironment.window.Node,
	Element: browserEnvironment.window.Element,
	HTMLElement: browserEnvironment.window.HTMLElement,
	Event: browserEnvironment.window.Event,
	KeyboardEvent: browserEnvironment.window.KeyboardEvent,
	MouseEvent: browserEnvironment.window.MouseEvent,
})) {
	Object.defineProperty(globalThis, name, { configurable: true, value });
}

const { View } = await import('../../../../browser/view.js');
const { EditorLineWrapping } = await import('../../../../common/config/editorOptions.js');
const { EditorZoneWidget } = await import('../../browser/zoneWidget.js');

test('EditorZoneWidget reserves editor space, tracks its anchor, updates layout, and releases its zone', () => {
	const dom = new JSDOM('<!doctype html><body><main></main></body>');
	using model = new TextModel('alpha\nbeta\ngamma');
	using viewport = new View({
		container: requiredElement<HTMLElement>(dom.window.document, 'main'),
		model,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
	});
	viewport.layout({ width: 200, height: 40 });
	const revealedRanges: Range[] = [];
	const editor: ZoneWidgetEditor = {
		viewport,
		revealRange: range => {
			revealedRanges.push(range);
			viewport.revealPosition(range.getStartPosition());
		},
	};
	using widget = new TestZoneWidget(editor, {
		className: 'peek-widget test-widget',
		frameWidth: 2,
		frameColor: '#123456',
		arrowColor: '#654321',
		ordinal: 20,
	});
	widget.create();

	widget.show(new Position((1) + 1, (2) + 1), 2);

	assert.deepEqual({
		position: widget.position,
		parent: widget.domNode.parentElement,
		top: widget.domNode.style.top,
		height: widget.domNode.style.height,
		contentHeight: viewport.viewportLayout.contentSize.height,
		classes: [...widget.domNode.classList],
		layout: widget.layouts.at(-1),
		frameColor: widget.domNode.style.getPropertyValue('--stanza-zone-widget-frame-color'),
		arrowColor: widget.domNode.style.getPropertyValue('--stanza-zone-widget-arrow-color'),
		revealedRanges,
	}, {
		position: new Position((1) + 1, (2) + 1),
		parent: requiredElement(viewport.element, '.stanza-editor-view-zones'),
		top: '40px',
		height: '40px',
		contentHeight: 100,
		classes: ['stanza-editor-zone-widget', 'peek-widget', 'test-widget', 'show-frame', 'show-arrow', 'stanza-editor-view-zone'],
		layout: { heightInPixels: 22, widthInPixels: 200 },
		frameColor: '#123456',
		arrowColor: '#654321',
		revealedRanges: [Range.fromPositions(new Position((1) + 1, (2) + 1))],
	});
	viewport.setLineHeight(30);
	assert.deepEqual({ height: widget.domNode.style.height, layout: widget.layouts.at(-1) }, {
		height: '60px',
		layout: { heightInPixels: 36, widthInPixels: 200 },
	});
	viewport.setLineHeight(20);

	model.applyEdits([{ range: Range.fromPositions(new Position((0) + 1, (0) + 1)), text: 'new\n' }]);
	assert.deepEqual({ position: widget.position, top: widget.domNode.style.top }, {
		position: new Position((2) + 1, (2) + 1),
		top: '60px',
	});

	widget.updatePositionAndHeight(new Position((0) + 1, (1) + 1), 1);
	assert.deepEqual({
		position: widget.position,
		top: widget.domNode.style.top,
		height: widget.domNode.style.height,
		contentHeight: viewport.viewportLayout.contentSize.height,
		layout: widget.layouts.at(-1),
	}, {
		position: new Position((0) + 1, (1) + 1),
		top: '20px',
		height: '20px',
		contentHeight: 100,
		layout: { heightInPixels: 2, widthInPixels: 200 },
	});

	widget.style({ frameColor: '#abcdef', arrowColor: null });
	assert.equal(widget.domNode.style.getPropertyValue('--stanza-zone-widget-frame-color'), '#abcdef');
	assert.equal(widget.domNode.style.getPropertyValue('--stanza-zone-widget-arrow-color'), '');

	widget.hide();
	assert.deepEqual({
		position: widget.position,
		parent: widget.domNode.parentElement,
		hidden: widget.domNode.hidden,
		contentHeight: viewport.viewportLayout.contentSize.height,
	}, {
		position: undefined,
		parent: null,
		hidden: true,
		contentHeight: 80,
	});
	dom.window.close();
});

test('EditorZoneWidget preserves selection on request and exposes an enabled resize sash while shown', () => {
	const dom = new JSDOM('<!doctype html><body><main></main></body>');
	using model = new TextModel('alpha\nbeta');
	using viewport = new View({
		container: requiredElement<HTMLElement>(dom.window.document, 'main'),
		model,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
	});
	viewport.layout({ width: 200, height: 40 });
	let revealRangeCount = 0;
	const editor: ZoneWidgetEditor = {
		viewport,
		revealRange: () => { revealRangeCount += 1; },
	};
	using widget = new TestZoneWidget(editor, {
		showFrame: false,
		showArrow: false,
		isAccessible: true,
		isResizable: true,
		keepEditorSelection: true,
	});
	widget.create();
	widget.show(new Position((0) + 1, (0) + 1), 6);

	const sash = requiredElement<HTMLElement>(widget.domNode, '.zeta-sash-horizontal');
	assert.deepEqual({
		revealRangeCount,
		ariaHidden: widget.domNode.getAttribute('aria-hidden'),
		role: widget.domNode.getAttribute('role'),
		height: widget.domNode.style.height,
		sashHidden: sash.hidden,
		sashDisabled: sash.getAttribute('aria-disabled'),
	}, {
		revealRangeCount: 0,
		ariaHidden: null,
		role: null,
		height: '120px',
		sashHidden: false,
		sashDisabled: 'false',
	});

	widget.resizeTo(8);
	assert.deepEqual({ height: widget.domNode.style.height, layout: widget.layouts.at(-1) }, {
		height: '160px',
		layout: { heightInPixels: 160, widthInPixels: 200 },
	});
	dom.window.close();
});

test('EditorZoneWidget places an anchor after its wrapped visual line', () => {
	const dom = new JSDOM('<!doctype html><body><main></main></body>');
	using model = new TextModel('abcdefghijklmnopqrst');
	using viewport = new View({
		container: requiredElement<HTMLElement>(dom.window.document, 'main'),
		model,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
	});
	viewport.layout({ width: 100, height: 40 });
	viewport.setLineWrapping(EditorLineWrapping.On);
	const anchor = new Position((0) + 1, (18) + 1);
	const visualLineIndex = viewport.getVisualLineProjection().visualLineIndexAt(anchor);
	assert.ok(visualLineIndex > 0);
	const editor: ZoneWidgetEditor = { viewport, revealRange: range => viewport.revealPosition(range.getStartPosition()) };
	using widget = new TestZoneWidget(editor, { showFrame: false, showArrow: false, keepEditorSelection: true });
	widget.create();

	widget.show(anchor, 1);

	assert.equal(widget.domNode.style.top, `${(visualLineIndex + 1) * 20}px`);
	dom.window.close();
});

class TestZoneWidget extends EditorZoneWidget {
	public readonly layouts: Array<{ readonly heightInPixels: number; readonly widthInPixels: number }> = [];

	public resizeTo(heightInLines: number): void {
		this.relayout(heightInLines);
	}

	protected override fillContainer(container: HTMLElement): void {
		const content = h(container.ownerDocument, 'button');
		content.textContent = 'Content';
		container.append(content);
	}

	protected override layoutContent(heightInPixels: number, widthInPixels: number): void {
		this.layouts.push({ heightInPixels, widthInPixels });
	}
}

function requiredElement<T extends Element = HTMLElement>(root: ParentNode, selector: string): T {
	const element = root.querySelector<T>(selector);
	assert.ok(element);
	return element;
}

class FixedTextMeasurer implements TextMeasurer {
	public readonly horizontalPadding = 24;
	public readonly contentLeftPadding = 12;

	public refresh(): boolean {
		return false;
	}

	public measureLineWidth(text: string): number {
		return text.length * 10;
	}
}
