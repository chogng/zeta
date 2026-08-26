import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { type TextMeasurer } from "../../../../browser/config/fontMeasurements.js";
import { TextModel } from "../../../../common/model/textModel.js";
import { h } from "../../../../../base/browser/dom.js";

const browserEnvironment = new JSDOM("<!doctype html><body></body>");
for (const [name, value] of Object.entries({
	window: browserEnvironment.window,
	document: browserEnvironment.window.document,
	Node: browserEnvironment.window.Node,
	Element: browserEnvironment.window.Element,
	HTMLElement: browserEnvironment.window.HTMLElement,
	Event: browserEnvironment.window.Event,
	KeyboardEvent: browserEnvironment.window.KeyboardEvent,
})) {
	Object.defineProperty(globalThis, name, { configurable: true, value });
}

const { EditorViewport } = await import("../../../../browser/view.js");
const { ToggleTabFocusModeController } = await import("../../browser/toggleTabFocusModeController.js");
const { TabFocus } = await import("../../../../browser/config/tabFocus.js");

test("Tab focus mode exposes state through Stanza-owned data and an accessibility announcement", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector<HTMLElement>("main")!;
	using model = new TextModel("text");
	using viewport = new EditorViewport({ container, model, lineHeight: 20, textMeasurer: new FixedTextMeasurer() });
	const input = h(dom.window.document, "textarea");
	container.append(input);
	using tabFocus = new TabFocus();
	using controller = new ToggleTabFocusModeController(input, viewport, tabFocus);

	assert.equal(viewport.element.getAttribute("role"), "region");
	assert.equal(viewport.element.hasAttribute("aria-pressed"), false);
	assert.equal(viewport.element.dataset.tabFocusMode, "false");

	const toggle = keydown(dom.window, "m");
	input.dispatchEvent(toggle);
	assert.equal(toggle.defaultPrevented, true);
	assert.equal(controller.isEnabled, true);
	assert.equal(viewport.element.dataset.tabFocusMode, "true");
	assert.equal(viewport.element.querySelector(".stanza-editor-accessibility-status")?.textContent, "Tab moves focus out of the editor");
	assert.equal(viewport.element.hasAttribute("aria-pressed"), false);
	tabFocus.setEnabled(false);
	assert.equal(controller.isEnabled, false);
	assert.equal(viewport.element.dataset.tabFocusMode, "false");
	dom.window.close();
});

class FixedTextMeasurer implements TextMeasurer {
	readonly horizontalPadding = 24;
	readonly contentLeftPadding = 12;

	refresh(): boolean {
		return false;
	}

	measureLineWidth(text: string): number {
		return text.length * 10;
	}
}

function keydown(targetWindow: typeof browserEnvironment.window, key: string): KeyboardEvent {
	return new targetWindow.KeyboardEvent("keydown", {
		bubbles: true,
		cancelable: true,
		key,
		ctrlKey: true,
	}) as unknown as KeyboardEvent;
}
