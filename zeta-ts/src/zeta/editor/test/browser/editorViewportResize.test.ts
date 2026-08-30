import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { EditorZoom } from "../../common/config/editorZoom.js";
import { TextModel } from "../../common/model/textModel.js";

const browserEnvironment = new JSDOM("<!doctype html><body></body>");
for (const [name, value] of Object.entries({
	window: browserEnvironment.window,
	document: browserEnvironment.window.document,
	Node: browserEnvironment.window.Node,
	Element: browserEnvironment.window.Element,
	HTMLElement: browserEnvironment.window.HTMLElement,
	Event: browserEnvironment.window.Event,
})) {
	Object.defineProperty(globalThis, name, { configurable: true, value });
}

const { View } = await import("../../browser/view.js");

test.after(() => browserEnvironment.window.close());

test("Stanza viewport automatic layout uses the observed content box", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	let resizeListener: ResizeObserverCallback | undefined;
	class TestResizeObserver {
		constructor(listener: ResizeObserverCallback) {
			resizeListener = listener;
		}

		observe(): void {}
		unobserve(): void {}
		disconnect(): void {}
	}
	Object.defineProperty(dom.window, "ResizeObserver", { configurable: true, value: TestResizeObserver });
	Object.defineProperty(globalThis, "ResizeObserver", { configurable: true, value: TestResizeObserver });
	const container = requiredElement(dom.window.document, "main");
	using model = new TextModel();
	using viewport = new View({ container, model, lineHeight: 20, automaticLayout: true });
	Object.defineProperties(viewport.element, {
		clientWidth: { configurable: true, value: 383 },
		clientHeight: { configurable: true, value: 62 },
	});

	resizeListener?.([{ contentRect: { width: 383.3875, height: 46.7875 } } as ResizeObserverEntry], {} as ResizeObserver);

	assert.deepEqual(viewport.viewportLayout.viewportSize, { width: 383, height: 46 });
	assert.equal(viewport.element.classList.contains("horizontally-scrollable"), false);
	assert.equal(viewport.element.classList.contains("vertically-scrollable"), false);
	assert.equal(requiredElement<HTMLElement>(viewport.element, ".stanza-editor-scrollbar-track-horizontal").hidden, true);
	assert.equal(requiredElement<HTMLElement>(viewport.element, ".stanza-editor-scrollbar-track-vertical").hidden, true);
	assert.equal(requiredElement<HTMLElement>(viewport.element, ".stanza-editor-content").style.width, "383px");
	assert.equal(requiredElement<HTMLElement>(viewport.element, ".stanza-editor-content").style.height, "46px");
	dom.window.close();
});

test("Stanza viewport enables scrollbars only for model-backed overflow", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	const container = requiredElement(dom.window.document, "main");
	using model = new TextModel(`${"x".repeat(100)}\nsecond line`);
	using viewport = new View({ container, model, lineHeight: 20 });

	viewport.layout({ width: 50, height: 20 });

	assert.equal(viewport.element.classList.contains("horizontally-scrollable"), true);
	assert.equal(viewport.element.classList.contains("vertically-scrollable"), true);
	const horizontalScrollbar = requiredElement<HTMLElement>(viewport.element, ".stanza-editor-scrollbar-track-horizontal");
	const verticalScrollbar = requiredElement<HTMLElement>(viewport.element, ".stanza-editor-scrollbar-track-vertical");
	assert.equal(horizontalScrollbar.hidden, false);
	assert.equal(verticalScrollbar.hidden, false);
	assert.equal(horizontalScrollbar.getAttribute("role"), "scrollbar");
	assert.equal(verticalScrollbar.getAttribute("aria-controls"), viewport.element.id);
	assert.equal(viewport.element.style.getPropertyValue("--stanza-editor-horizontal-scrollbar-size"), "12px");
	assert.equal(viewport.element.style.getPropertyValue("--stanza-editor-vertical-scrollbar-size"), "14px");
	assert.equal(horizontalScrollbar.style.right, "14px");
	assert.equal(verticalScrollbar.style.bottom, "12px");
	dom.window.close();
});

test("Stanza viewport applies recomputed font configuration", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	const container = requiredElement(dom.window.document, "main");
	using model = new TextModel("text");
	const viewport = new View({ container, model, lineHeight: 20, minimap: { enabled: false } });
	try {
		EditorZoom.setZoomLevel(1);
		assert.equal(viewport.fontInfo.lineHeight, 22);
		assert.equal(viewport.currentLayout.lineHeight, 22);
		assert.equal(viewport.element.style.lineHeight, "22px");
	} finally {
		viewport.dispose();
		EditorZoom.setZoomLevel(0);
		dom.window.close();
	}
});

function requiredElement<T extends Element = HTMLElement>(root: ParentNode, selector: string): T {
	const element = root.querySelector<T>(selector);
	assert.ok(element);
	return element;
}
