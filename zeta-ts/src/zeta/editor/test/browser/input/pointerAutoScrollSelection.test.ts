import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { type TextMeasurer } from "../../../browser/measurement/fontMetrics.js";
import { EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { TextSelection, TextSelectionSet } from "../../../common/core/selection.js";
import { TextPosition } from "../../../common/core/text.js";
import { TextModel } from "../../../common/model/textModel.js";

class FixedTextMeasurer implements TextMeasurer {
	readonly horizontalPadding = 24;
	readonly contentLeftPadding = 12;

	refresh(): boolean {
		return false;
	}

	measureLineWidth(text: string): number {
		return [...text].length * 10;
	}
}

class FakeAnimationFrames {
	private readonly callbacks = new Map<number, FrameRequestCallback>();
	private nextHandle = 1;
	private now = 0;

	install(targetWindow: typeof browserEnvironment.window): void {
		Object.defineProperty(targetWindow, "requestAnimationFrame", {
			configurable: true,
			value: (callback: FrameRequestCallback): number => {
				const handle = this.nextHandle++;
				this.callbacks.set(handle, callback);
				return handle;
			},
		});
		Object.defineProperty(targetWindow, "cancelAnimationFrame", {
			configurable: true,
			value: (handle: number): void => {
				this.callbacks.delete(handle);
			},
		});
		Object.defineProperty(targetWindow.performance, "now", {
			configurable: true,
			value: (): number => this.now,
		});
	}

	get pendingCount(): number {
		return this.callbacks.size;
	}

	flush(duration = 1_000 / 60): void {
		const callbacks = [...this.callbacks.values()];
		this.callbacks.clear();
		this.now += duration;
		for (const callback of callbacks) callback(this.now);
	}
}

const browserEnvironment = new JSDOM("<!doctype html><body></body>");
for (const [name, value] of Object.entries({
	window: browserEnvironment.window,
	document: browserEnvironment.window.document,
	Node: browserEnvironment.window.Node,
	Element: browserEnvironment.window.Element,
	HTMLElement: browserEnvironment.window.HTMLElement,
	Event: browserEnvironment.window.Event,
})) {
	Object.defineProperty(globalThis, name, {
		configurable: true,
		value,
	});
}

const { EditorViewport } = await import("../../../browser/view/editorViewport.js");
const { PointerSelectionController } = await import("../../../browser/input/pointerSelectionController.js");

test("Pointer drag autoscroll advances selection and stops at boundaries", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const frames = new FakeAnimationFrames();
	frames.install(dom.window);
	const container = dom.window.document.querySelector("main");
	assert.ok(container);
	using model = new TextModel(
		Array.from({ length: 20 }, () => "abcdefghij").join("\n"),
	);
	using selections = new EditorSelectionController(
		model,
		TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 0))),
	);
	using viewport = new EditorViewport({
		container,
		model,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
		selectionController: selections,
	});
	viewport.layout({ width: 100, height: 40 });
	viewport.element.getBoundingClientRect = () => editorBounds();
	using pointer = new PointerSelectionController(viewport, selections);

	viewport.element.dispatchEvent(pointerEvent(
		dom.window,
		"pointerdown",
		158,
		55,
		1,
	));
	dom.window.dispatchEvent(pointerEvent(
		dom.window,
		"pointermove",
		158,
		110,
		1,
	));
	assert.deepEqual(selections.selections.primary, TextSelection.from(
		TextPosition.at(0, 1),
		TextPosition.at(1, 1),
	));
	assert.equal(frames.pendingCount, 1);

	frames.flush();
	assert.equal(viewport.viewportLayout.scrollPosition.top, 10);
	assert.deepEqual(selections.selections.primary, TextSelection.from(
		TextPosition.at(0, 1),
		TextPosition.at(2, 1),
	));

	dom.window.dispatchEvent(pointerEvent(
		dom.window,
		"pointerup",
		158,
		110,
		1,
	));
	const stoppedTop = viewport.viewportLayout.scrollPosition.top;
	frames.flush();
	assert.equal(viewport.viewportLayout.scrollPosition.top, stoppedTop);

	viewport.scrollTo({ left: 0, top: 0 });
	viewport.element.dispatchEvent(pointerEvent(
		dom.window,
		"pointerdown",
		158,
		55,
		2,
	));
	dom.window.dispatchEvent(pointerEvent(
		dom.window,
		"pointermove",
		220,
		55,
		2,
	));
	assert.deepEqual(selections.selections.primary, TextSelection.from(
		TextPosition.at(0, 1),
		TextPosition.at(0, 5),
	));

	let frameCount = 0;
	while (frames.pendingCount > 0 && frameCount < 20) {
		frames.flush();
		frameCount += 1;
	}
	assert.equal(viewport.viewportLayout.scrollPosition.left, 60);
	assert.equal(frames.pendingCount, 0);
	assert.deepEqual(selections.selections.primary, TextSelection.from(
		TextPosition.at(0, 1),
		TextPosition.at(0, 10),
	));

	dom.window.dispatchEvent(pointerEvent(
		dom.window,
		"pointerup",
		220,
		55,
		2,
	));

	viewport.scrollTo({ left: 0, top: 0 });
	viewport.element.dispatchEvent(pointerEvent(
		dom.window,
		"pointerdown",
		158,
		55,
		3,
	));
	dom.window.dispatchEvent(pointerEvent(
		dom.window,
		"pointermove",
		220,
		55,
		3,
	));
	dom.window.dispatchEvent(pointerEvent(
		dom.window,
		"pointercancel",
		220,
		55,
		3,
	));
	frames.flush();
	assert.deepEqual(viewport.viewportLayout.scrollPosition, {
		left: 0,
		top: 0,
	});

	viewport.element.dispatchEvent(pointerEvent(
		dom.window,
		"pointerdown",
		158,
		55,
		4,
	));
	dom.window.dispatchEvent(pointerEvent(
		dom.window,
		"pointermove",
		220,
		55,
		4,
	));
	dom.window.dispatchEvent(pointerEvent(
		dom.window,
		"pointermove",
		158,
		55,
		4,
	));
	frames.flush();
	assert.deepEqual(viewport.viewportLayout.scrollPosition, {
		left: 0,
		top: 0,
	});
	dom.window.dispatchEvent(pointerEvent(
		dom.window,
		"pointerup",
		158,
		55,
		4,
	));

	dom.window.close();
});

function pointerEvent(
	targetWindow: typeof browserEnvironment.window,
	type: string,
	clientX: number,
	clientY: number,
	pointerId: number,
): PointerEvent {
	const event = new targetWindow.MouseEvent(type, {
		bubbles: true,
		cancelable: true,
		button: 0,
		buttons: type === "pointerup" || type === "pointercancel" ? 0 : 1,
		clientX,
		clientY,
	});
	Object.defineProperty(event, "pointerId", {
		configurable: true,
		value: pointerId,
	});
	return event as unknown as PointerEvent;
}

function editorBounds(): DOMRect {
	return {
		x: 100,
		y: 50,
		left: 100,
		top: 50,
		right: 200,
		bottom: 90,
		width: 100,
		height: 40,
		toJSON: () => ({}),
	};
}
