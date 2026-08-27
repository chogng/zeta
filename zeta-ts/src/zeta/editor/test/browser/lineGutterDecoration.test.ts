import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { Emitter } from "../../../base/common/event.js";
import { EditorLineGutterRenderer, type EditorLineGutterDecoration, type EditorLineGutterItem } from "../../browser/viewparts/margin/lineGutterDecoration.js";

test("gutter renderer owns stable DOM and routes provider state and activation", () => {
	const dom = new JSDOM("<!doctype html><body></body>");
	const host = dom.window.document.body;
	using first = new TestDecoration("first");
	using second = new TestDecoration("second");
	let changes = 0;
	using renderer = new EditorLineGutterRenderer(host, [first, second], () => changes += 1);
	const element = renderer.create(dom.window.document);
	host.append(element);

	renderer.render(element, 6, true);
	const buttons = [...element.querySelectorAll("button")];

	assert.equal(renderer.width, 40);
	assert.deepEqual(buttons.map(button => ({ label: button.ariaLabel, line: button.dataset.logicalLineIndex, className: button.className })), [
		{ label: "first at 7", line: "6", className: "stanza-editor-gutter-decoration first" },
		{ label: "second at 7", line: "6", className: "stanza-editor-gutter-decoration second" },
	]);
	buttons[1]!.click();
	assert.equal(second.activatedLineIndex, 6);
	first.fireChange();
	assert.equal(changes, 1);
	renderer.render(element, 7, false);
	assert.equal(buttons[0], element.querySelectorAll("button")[0]);
	assert.equal(buttons[0]!.hidden, true);
	dom.window.close();
});

class TestDecoration implements EditorLineGutterDecoration {
	private readonly emitter = new Emitter<void>();
	readonly onDidChange = this.emitter.event;
	activatedLineIndex: number | undefined;
	constructor(private readonly label: string) {}
	getDecoration(logicalLineIndex: number, firstForLogicalLine: boolean): EditorLineGutterItem | undefined {
		return firstForLogicalLine ? { className: this.label, label: `${this.label} at ${logicalLineIndex + 1}` } : undefined;
	}
	activate(logicalLineIndex: number): void { this.activatedLineIndex = logicalLineIndex; }
	fireChange(): void { this.emitter.fire(); }
	dispose(): void { this.emitter.dispose(); }
	[Symbol.dispose](): void { this.dispose(); }
}
