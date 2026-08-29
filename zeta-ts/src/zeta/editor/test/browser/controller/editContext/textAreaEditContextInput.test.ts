import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { h } from "../../../../../base/browser/dom.js";
import { TextAreaInput } from "../../../../browser/controller/editContext/textArea/textAreaEditContextInput.js";
import { TextAreaEditContext } from "../../../../browser/controller/editContext/textArea/textAreaEditContext.js";
import { TextAreaEditContextRegistry } from "../../../../browser/controller/editContext/textArea/textAreaEditContextRegistry.js";
import { CursorsController } from '../../../../common/cursor/cursor.js';
import { Selection } from '../../../../common/core/selection.js';
import { SelectionSet } from '../../../../common/cursor/selectionSet.js';
import { Position } from '../../../../common/core/position.js';
import { TextModel } from '../../../../common/model/textModel.js';
import { type TextMeasurer } from '../../../../browser/config/fontMeasurements.js';
import { EditorView, EditorViewport } from '../../../../browser/view.js';

test("TextAreaInput owns textarea DOM events and direction-aware state", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const textarea = h(dom.window.document, "textarea");
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

test("TextAreaEditContext delegates to TextAreaInput and registers its domNode", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector("main")!;
	using model = new TextModel('hello');
	using selections = new CursorsController(
		model,
		SelectionSet.single(Selection.fromPositions(new Position((0) + 1, (0) + 1))),
	);
	using viewport = new EditorViewport({
		container,
		model,
		lineHeight: 20,
		selectionController: selections,
		textMeasurer: new FixedTextMeasurer(),
	});
	using editor = new EditorView(viewport, selections);
	const context = editor.editContext;
	assert.ok(context instanceof TextAreaEditContext);
	assert.equal(context.textAreaInput.element, context.domNode);
	assert.equal(TextAreaEditContextRegistry.get(context.domNode), context);
	assert.equal(Position.compare(context.getLastRenderData()!, new Position(1, 1)), 0);
	editor.setAriaOptions({ activeDescendant: 'completion-option' });
	assert.equal(context.domNode.getAttribute('aria-autocomplete'), 'list');
	assert.equal(context.domNode.getAttribute('aria-activedescendant'), 'completion-option');
	editor.focus();
	assert.equal(editor.isFocused(), true);

	context.setValue("test", "hello");
	context.setSelectionRange("test", 4, 1);
	assert.equal(context.getValue(), "hello");
	assert.equal(context.getSelectionStart(), 4);
	assert.equal(context.getSelectionEnd(), 1);

	editor.dispose();
	assert.equal(TextAreaEditContextRegistry.get(context.domNode), undefined);
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
