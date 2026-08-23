import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { type TextMeasurer } from "../../browser/measurement/fontMetrics.js";
import { EditorHitTargetKind, hitTestAsterEditorPoint } from "../../common/viewModel/pointerHitTest.js";
import { TextPosition } from "../../common/core/text.js";
import { TextModel } from "../../common/model/textModel.js";

class FixedTextMeasurer implements TextMeasurer {
	readonly horizontalPadding = 24;
	readonly contentLeftPadding = 12;

	refresh(): boolean {
		return false;
	}

	measureLineWidth(text: string): number {
		let width = 0;
		for (const character of text) {
			if (character === "\t") {
				width = (Math.floor(width / 40) + 1) * 40;
			} else {
				width += 10;
			}
		}
		return width;
	}
}

test("Pointer hit testing distinguishes gutter, text, empty content, and lines", () => {
	using model = new TextModel("a\tb\n😀a\n");
	const measurer = new FixedTextMeasurer();
	const layout = {
		lineHeight: 20,
		viewportSize: { width: 200, height: 80 },
		scrollPosition: { left: 0, top: 0 },
	};
	const metrics = { gutterWidth: 30, textLeft: 40 };
	const hit = (left: number, top: number) => hitTestAsterEditorPoint(
		model,
		layout,
		{ left, top },
		metrics,
		measurer,
	);

	assert.equal(hit(-1, 0), undefined);
	assert.equal(hit(0, 80), undefined);
	assert.throws(() => hitTestAsterEditorPoint(
		model,
		{ ...layout, lineHeight: 0 },
		{ left: 0, top: 0 },
		metrics,
		measurer,
	), /layout is invalid/);
	assert.deepEqual(hit(10, 25), {
		kind: EditorHitTargetKind.Gutter,
		position: TextPosition.at(1, 0),
	});
	assert.deepEqual(hit(35, 5), {
		kind: EditorHitTargetKind.EmptyContent,
		position: TextPosition.at(0, 0),
	});
	assert.deepEqual(hit(54, 5), {
		kind: EditorHitTargetKind.Text,
		position: TextPosition.at(0, 1),
	});
	assert.deepEqual(hit(65, 5), {
		kind: EditorHitTargetKind.Text,
		position: TextPosition.at(0, 2),
	});
	assert.deepEqual(hit(86, 5), {
		kind: EditorHitTargetKind.Text,
		position: TextPosition.at(0, 3),
	});
	assert.deepEqual(hit(100, 5), {
		kind: EditorHitTargetKind.EmptyContent,
		position: TextPosition.at(0, 3),
	});
	assert.deepEqual(hit(44, 25), {
		kind: EditorHitTargetKind.Text,
		position: TextPosition.at(1, 0),
	});
	assert.deepEqual(hit(45, 25), {
		kind: EditorHitTargetKind.Text,
		position: TextPosition.at(1, 2),
	});
	assert.deepEqual(hit(40, 45), {
		kind: EditorHitTargetKind.EmptyContent,
		position: TextPosition.at(2, 0),
	});
	assert.deepEqual(hit(40, 65), {
		kind: EditorHitTargetKind.AfterLines,
		position: TextPosition.at(2, 0),
	});
});

test("Pointer hit testing applies sticky gutter and viewport scrolling", () => {
	using model = new TextModel("first\n😀a\nthird");
	const layout = {
		lineHeight: 20,
		viewportSize: { width: 100, height: 40 },
		scrollPosition: { left: 20, top: 20 },
	};
	const metrics = { gutterWidth: 30, textLeft: 40 };

	assert.deepEqual(hitTestAsterEditorPoint(
		model,
		layout,
		{ left: 10, top: 5 },
		metrics,
		new FixedTextMeasurer(),
	), {
		kind: EditorHitTargetKind.Gutter,
		position: TextPosition.at(1, 0),
	});
	assert.deepEqual(hitTestAsterEditorPoint(
		model,
		layout,
		{ left: 30, top: 5 },
		metrics,
		new FixedTextMeasurer(),
	), {
		kind: EditorHitTargetKind.Text,
		position: TextPosition.at(1, 2),
	});
});

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

const { EditorViewport } = await import(
	"../../browser/view/editorViewport.js"
);

test("Viewport maps client coordinates through its bounds and scroll state", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector("main");
	assert.ok(container);
	using model = new TextModel(
		Array.from({ length: 20 }, () => "abcdefghij".repeat(3)).join("\n"),
	);
	using viewport = new EditorViewport({
		container,
		model,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
	});
	viewport.layout({ width: 100, height: 40 });
	viewport.scrollTo({ left: 20, top: 20 });
	viewport.element.getBoundingClientRect = () => ({
		x: 100,
		y: 50,
		left: 100,
		top: 50,
		right: 200,
		bottom: 90,
		width: 100,
		height: 40,
		toJSON: () => ({}),
	});

	assert.deepEqual(viewport.getTargetAtClientPoint({
		clientX: 110,
		clientY: 55,
	}), {
		kind: EditorHitTargetKind.Gutter,
		position: TextPosition.at(1, 0),
	});
	assert.deepEqual(viewport.getTargetAtClientPoint({
		clientX: 140,
		clientY: 55,
	}), {
		kind: EditorHitTargetKind.Text,
		position: TextPosition.at(1, 1),
	});
	assert.equal(viewport.getTargetAtClientPoint({
		clientX: 99,
		clientY: 55,
	}), undefined);
	assert.deepEqual(viewport.getNearestTargetAtClientPoint({
		clientX: 50,
		clientY: 55,
	}), {
		kind: EditorHitTargetKind.Gutter,
		position: TextPosition.at(1, 0),
	});
	assert.deepEqual(viewport.getNearestTargetAtClientPoint({
		clientX: 140,
		clientY: 100,
	}), {
		kind: EditorHitTargetKind.Text,
		position: TextPosition.at(2, 1),
	});
	assert.throws(() => viewport.getTargetAtClientPoint({
		clientX: Number.NaN,
		clientY: 55,
	}), /finite coordinates/);

	dom.window.close();
});
