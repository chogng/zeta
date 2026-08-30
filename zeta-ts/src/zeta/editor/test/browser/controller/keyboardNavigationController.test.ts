import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { StandardKeyboardEvent } from "../../../../base/browser/keyboardEvent.js";
import { OperatingSystem } from "../../../../base/common/platform.js";
import { type TextMeasurer } from "../../../browser/config/fontMeasurements.js";
import { resolveStanzaKeyboardNavigation } from "../../../browser/view/viewController.js";
import { ViewUserInputEvents } from "../../../browser/view/viewUserInputEvents.js";
import { EditorCursorNavigationCommand, EditorCursorNavigationMode } from "../../../common/cursor/cursorMoveOperations.js";
import { CursorsController } from "../../../common/cursor/cursor.js";
import { Selection } from "../../../common/core/selection.js";
import { SelectionSet } from "../../../common/cursor/selectionSet.js";
import { Position } from "../../../common/core/position.js";
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

test("Keyboard navigation resolves Windows/Linux and macOS chords", () => {
	assert.deepEqual(resolveStanzaKeyboardNavigation(
		key("ArrowLeft"),
		OperatingSystem.Windows,
	), {
		command: EditorCursorNavigationCommand.CharacterLeft,
		mode: EditorCursorNavigationMode.Move,
	});
	assert.deepEqual(resolveStanzaKeyboardNavigation(
		key("End", { shiftKey: true }),
		OperatingSystem.Linux,
	), {
		command: EditorCursorNavigationCommand.LineEnd,
		mode: EditorCursorNavigationMode.Extend,
	});
	assert.deepEqual(resolveStanzaKeyboardNavigation(
		key("ArrowRight", { ctrlKey: true }),
		OperatingSystem.Windows,
	), {
		command: EditorCursorNavigationCommand.WordRight,
		mode: EditorCursorNavigationMode.Move,
	});
	assert.deepEqual(resolveStanzaKeyboardNavigation(
		key("Home", { ctrlKey: true, shiftKey: true }),
		OperatingSystem.Windows,
	), {
		command: EditorCursorNavigationCommand.DocumentStart,
		mode: EditorCursorNavigationMode.Extend,
	});
	assert.deepEqual(resolveStanzaKeyboardNavigation(
		key("ArrowLeft", { altKey: true, shiftKey: true }),
		OperatingSystem.Macintosh,
	), {
		command: EditorCursorNavigationCommand.WordLeft,
		mode: EditorCursorNavigationMode.Extend,
	});
	assert.deepEqual(resolveStanzaKeyboardNavigation(
		key("ArrowRight", { metaKey: true }),
		OperatingSystem.Macintosh,
	), {
		command: EditorCursorNavigationCommand.LineEnd,
		mode: EditorCursorNavigationMode.Move,
	});
	assert.deepEqual(resolveStanzaKeyboardNavigation(
		key("ArrowDown", { metaKey: true }),
		OperatingSystem.Macintosh,
	), {
		command: EditorCursorNavigationCommand.DocumentEnd,
		mode: EditorCursorNavigationMode.Move,
	});
	assert.equal(resolveStanzaKeyboardNavigation(
		key("ArrowLeft", { altKey: true }),
		OperatingSystem.Windows,
	), undefined);
	assert.equal(resolveStanzaKeyboardNavigation(
		key("ArrowLeft", { isComposing: true }),
		OperatingSystem.Windows,
	), undefined);
	assert.equal(resolveStanzaKeyboardNavigation(
		key("ArrowLeft", { altGraphKey: true }),
		OperatingSystem.Windows,
	), undefined);
});

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
	Object.defineProperty(globalThis, name, {
		configurable: true,
		value,
	});
}

const { View } = await import("../../../browser/view.js");
const { KeyboardNavigationController } = await import("../../../browser/view/viewController.js");
const { EditorLineWrapping } = await import("../../../common/config/editorOptions.js");

test("Keyboard controller retains columns, routes multi-selection, and reveals primary", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector("main");
	assert.ok(container);
	using model = new TextModel([
		"abcdef",
		"x",
		"abcdef",
		"line",
		"line",
		"line",
		"line",
		"line",
		"line",
		"abcdefghijklmnopqrstuvwxyz",
	].join("\n"));
	using selections = new CursorsController(
		model,
		SelectionSet.single(caret(0, 5)),
	);
	using viewport = new View({
		container,
		model,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
		selectionController: selections,
	});
	viewport.layout({ width: 80, height: 40 });
	const userInputEvents = new ViewUserInputEvents();
	let forwardedKeyDownCount = 0;
	const previousKeyDownHandler = (): void => {
		forwardedKeyDownCount++;
	};
	userInputEvents.onKeyDown = previousKeyDownHandler;
	using keyboard = new KeyboardNavigationController(
		viewport,
		selections,
		userInputEvents,
		{ operatingSystem: OperatingSystem.Windows },
	);

	const firstDown = keyboardEvent(dom.window, "ArrowDown");
	emitKeyDown(userInputEvents, firstDown);
	emitKeyDown(userInputEvents, keyboardEvent(dom.window, "ArrowDown"));
	assert.equal(firstDown.defaultPrevented, true);
	assert.equal(forwardedKeyDownCount, 2);
	assert.deepEqual(selections.selections.primary, caret(2, 5));

	selections.setSelections(SelectionSet.single(caret(0, 2)));
	emitKeyDown(userInputEvents, keyboardEvent(dom.window, "ArrowDown"));
	emitKeyDown(userInputEvents, keyboardEvent(dom.window, "ArrowDown"));
	assert.deepEqual(selections.selections.primary, caret(2, 2));

	emitKeyDown(userInputEvents, keyboardEvent(
		dom.window,
		"ArrowRight",
		{ shiftKey: true },
	));
	assert.deepEqual(
		selections.selections.primary,
		Selection.fromPositions(new Position((2) + 1, (2) + 1), new Position((2) + 1, (3) + 1)),
	);

	const textKey = keyboardEvent(dom.window, "a");
	emitKeyDown(userInputEvents, textKey);
	assert.equal(textKey.defaultPrevented, false);
	assert.deepEqual(
		selections.selections.primary,
		Selection.fromPositions(new Position((2) + 1, (2) + 1), new Position((2) + 1, (3) + 1)),
	);

	emitKeyDown(userInputEvents, keyboardEvent(
		dom.window,
		"End",
		{ ctrlKey: true },
	));
	assert.deepEqual(selections.selections.primary, caret(9, 26));
	assert.equal(viewport.viewportLayout.scrollPosition.top, 160);
	assert.ok(viewport.viewportLayout.scrollPosition.left > 0);

	emitKeyDown(userInputEvents, keyboardEvent(
		dom.window,
		"Home",
		{ ctrlKey: true },
	));
	assert.deepEqual(selections.selections.primary, caret(0, 0));
	assert.deepEqual(viewport.viewportLayout.scrollPosition, {
		left: 0,
		top: 0,
	});

	emitKeyDown(userInputEvents, keyboardEvent(dom.window, "PageDown"));
	assert.deepEqual(selections.selections.primary, caret(2, 0));
	assert.equal(viewport.viewportLayout.scrollPosition.top, 20);
	emitKeyDown(userInputEvents, keyboardEvent(dom.window, "PageUp"));
	assert.deepEqual(selections.selections.primary, caret(0, 0));
	assert.equal(viewport.viewportLayout.scrollPosition.top, 0);

	selections.setSelections(SelectionSet.withPrimary([
		caret(0, 1),
		caret(1, 1),
	], 1));
	emitKeyDown(userInputEvents, keyboardEvent(dom.window, "ArrowDown"));
	assert.deepEqual(selections.selections, SelectionSet.withPrimary([
		caret(1, 1),
		caret(2, 1),
	], 1));

	keyboard.dispose();
	assert.equal(userInputEvents.onKeyDown, previousKeyDownHandler);
	const disposedSelections = selections.selections;
	emitKeyDown(userInputEvents, keyboardEvent(dom.window, "ArrowDown"));
	assert.equal(selections.selections, disposedSelections);

	dom.window.close();
});

test("Keyboard controller moves by measured visual rows when soft wrapping is enabled", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector("main");
	assert.ok(container);
	using model = new TextModel("abcdef\nghij");
	using selections = new CursorsController(
		model,
		SelectionSet.single(caret(0, 1)),
	);
	using viewport = new View({
		container,
		model,
		glyphMargin: false,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
		selectionController: selections,
		lineWrapping: EditorLineWrapping.On,
		minimap: { enabled: false },
	});
	viewport.layout({ width: 70, height: 40 });
	const userInputEvents = new ViewUserInputEvents();
	using keyboard = new KeyboardNavigationController(
		viewport,
		selections,
		userInputEvents,
		{ operatingSystem: OperatingSystem.Windows },
	);

	emitKeyDown(userInputEvents, keyboardEvent(dom.window, "ArrowDown"));
	assert.deepEqual(selections.selections.primary, caret(0, 3));
	emitKeyDown(userInputEvents, keyboardEvent(dom.window, "ArrowDown"));
	assert.deepEqual(selections.selections.primary, caret(0, 5));
	emitKeyDown(userInputEvents, keyboardEvent(dom.window, "ArrowDown"));
	assert.deepEqual(selections.selections.primary, caret(1, 1));
	emitKeyDown(userInputEvents, keyboardEvent(dom.window, "ArrowUp"));
	assert.deepEqual(selections.selections.primary, caret(0, 5));

	selections.setSelections(SelectionSet.single(caret(0, 1)));
	emitKeyDown(userInputEvents, keyboardEvent(dom.window, "PageDown"));
	assert.deepEqual(selections.selections.primary, caret(0, 5));
	selections.setSelections(SelectionSet.single(caret(0, 1)));
	emitKeyDown(userInputEvents, keyboardEvent(
		dom.window,
		"ArrowDown",
		{ shiftKey: true },
	));
	assert.deepEqual(
		selections.selections.primary,
		Selection.fromPositions(new Position((0) + 1, (1) + 1), new Position((0) + 1, (3) + 1)),
	);

	dom.window.close();
});

test('Keyboard controller applies sticky tab stops to indentation movement', () => {
	const dom = new JSDOM('<!doctype html><body><main></main></body>');
	const container = dom.window.document.querySelector('main');
	assert.ok(container);
	using model = new TextModel('        value');
	using selections = new CursorsController(model, SelectionSet.single(caret(0, 8)));
	using viewport = new View({ container, model, lineHeight: 20, textMeasurer: new FixedTextMeasurer(), selectionController: selections });
	const userInputEvents = new ViewUserInputEvents();
	using keyboard = new KeyboardNavigationController(
		viewport,
		selections,
		userInputEvents,
		{ operatingSystem: OperatingSystem.Windows, stickyTabStops: true, tabSize: 4 },
	);

	emitKeyDown(userInputEvents, keyboardEvent(dom.window, 'ArrowLeft'));
	assert.deepEqual(selections.selections.primary, caret(0, 4));

	dom.window.close();
});

test("Keyboard controller rejects cross-model wiring and invalid OS options", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector("main");
	assert.ok(container);
	using model = new TextModel("alpha");
	using otherModel = new TextModel("beta");
	using selections = new CursorsController(
		otherModel,
		SelectionSet.single(caret(0, 0)),
	);
	using ownSelections = new CursorsController(
		model,
		SelectionSet.single(caret(0, 0)),
	);
	using viewport = new View({
		container,
		model,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
	});

	assert.throws(
		() => new KeyboardNavigationController(viewport, selections, new ViewUserInputEvents()),
		/must share one text model/,
	);
	assert.throws(
		() => new KeyboardNavigationController(
			viewport,
			ownSelections,
			new ViewUserInputEvents(),
			{ operatingSystem: "plan9" as OperatingSystem },
		),
		/Unknown Stanza keyboard operating system/,
	);

	dom.window.close();
});

interface KeyOptions {
	readonly ctrlKey?: boolean;
	readonly shiftKey?: boolean;
	readonly altKey?: boolean;
	readonly metaKey?: boolean;
	readonly altGraphKey?: boolean;
	readonly isComposing?: boolean;
}

function emitKeyDown(userInputEvents: ViewUserInputEvents, event: KeyboardEvent): void {
	userInputEvents.emitKeyDown(new StandardKeyboardEvent(event));
}

function key(keyValue: string, options: KeyOptions = {}) {
	return {
		key: keyValue,
		ctrlKey: options.ctrlKey ?? false,
		shiftKey: options.shiftKey ?? false,
		altKey: options.altKey ?? false,
		metaKey: options.metaKey ?? false,
		altGraphKey: options.altGraphKey ?? false,
		isComposing: options.isComposing ?? false,
	};
}

function keyboardEvent(
	targetWindow: typeof browserEnvironment.window,
	keyValue: string,
	options: KeyOptions = {},
): KeyboardEvent {
	return new targetWindow.KeyboardEvent("keydown", {
		bubbles: true,
		cancelable: true,
		key: keyValue,
		ctrlKey: options.ctrlKey,
		shiftKey: options.shiftKey,
		altKey: options.altKey,
		metaKey: options.metaKey,
		isComposing: options.isComposing,
	}) as unknown as KeyboardEvent;
}

function caret(lineIndex: number, columnIndex: number): Selection {
	return Selection.fromPositions(new Position((lineIndex) + 1, (columnIndex) + 1));
}
