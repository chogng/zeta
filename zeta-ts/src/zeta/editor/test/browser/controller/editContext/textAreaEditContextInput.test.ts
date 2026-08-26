import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { TextAreaInput } from "../../../../browser/controller/editContext/textArea/textAreaEditContextInput.js";
import { TextAreaEditContext } from "../../../../browser/controller/editContext/textArea/textAreaEditContext.js";
import { TextAreaEditContextRegistry } from "../../../../browser/controller/editContext/textArea/textAreaEditContextRegistry.js";

test("TextAreaInput owns textarea DOM events and direction-aware state", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const textarea = dom.window.document.createElement("textarea");
	dom.window.document.querySelector("main")!.append(textarea);
	using input = new TextAreaInput(textarea);
	let focusCount = 0;
	let selectCount = 0;
	using focusListener = input.onDidFocus(() => focusCount += 1);
	using selectListener = input.onDidSelect(() => selectCount += 1);
	input.connect();

	textarea.focus();
	textarea.value = "abcd";
	textarea.setSelectionRange(1, 3, "backward");
	textarea.dispatchEvent(new dom.window.Event("select", { bubbles: true }));

	assert.equal(focusCount, 1);
	assert.equal(selectCount, 1);
	assert.equal(input.getSelectionStart(), 3);
	assert.equal(input.getSelectionEnd(), 1);
	assert.equal(input.textAreaState.value, "abcd");

	dom.window.close();
});

test("TextAreaEditContext delegates to TextAreaInput and registers its element", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector("main")!;
	using context = new TextAreaEditContext(container);
	assert.equal(context.textAreaInput.element, context.element);
	assert.equal(TextAreaEditContextRegistry.get(context.element), context);

	context.setValue("test", "hello");
	context.setSelectionRange("test", 4, 1);
	assert.equal(context.getValue(), "hello");
	assert.equal(context.getSelectionStart(), 4);
	assert.equal(context.getSelectionEnd(), 1);

	context.dispose();
	assert.equal(TextAreaEditContextRegistry.get(context.element), undefined);
	dom.window.close();
});
