import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { type EditorViewport } from "../../../browser/view/editorViewport.js";
import { MouseTargetFactory, MouseTargetKind } from "../../../browser/controller/mouseTarget.js";
import { TextPosition } from "../../../common/core/text.js";
import { EditorHitTargetKind } from "../../../common/viewModel/pointerHitTest.js";

test("MouseTargetFactory preserves editor targets and classifies browser-owned regions", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector("main");
	assert.ok(container);
	const editorTarget = Object.freeze({
		kind: EditorHitTargetKind.Text,
		position: TextPosition.at(0, 1),
	});
	const viewport = {
		element: container,
		getTargetAtClientPoint: () => editorTarget,
		getNearestTargetAtClientPoint: () => editorTarget,
	} as unknown as EditorViewport;
	const factory = new MouseTargetFactory(viewport);
	const node = (className: string): HTMLElement => {
		const element = dom.window.document.createElement("span");
		element.className = className;
		container.append(element);
		return element;
	};

	assert.equal(factory.create(mouseEvent(node("stanza-editor-line-number")))?.kind, MouseTargetKind.LineNumber);
	assert.equal(factory.create(mouseEvent(node("stanza-editor-feature-gutter-slot")))?.kind, MouseTargetKind.GutterDecoration);
	assert.equal(factory.create(mouseEvent(node("stanza-editor-scrollbar-track stanza-editor-scrollbar-track-vertical")))?.kind, MouseTargetKind.Scrollbar);
	assert.equal(factory.create(mouseEvent(node("stanza-editor-zone-widget")))?.kind, MouseTargetKind.ViewZone);
	assert.equal(factory.create(mouseEvent(node("stanza-editor-content-widget")))?.kind, MouseTargetKind.Widget);

	const textTarget = factory.create(mouseEvent(node("stanza-editor-text")));
	assert.equal(textTarget?.kind, MouseTargetKind.Text);
	assert.equal(textTarget?.editorTarget, editorTarget);
});

function mouseEvent(target: EventTarget): Pick<MouseEvent, "clientX" | "clientY" | "target"> {
	return { clientX: 10, clientY: 10, target };
}
