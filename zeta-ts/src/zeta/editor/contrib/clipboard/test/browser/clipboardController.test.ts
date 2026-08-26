import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { type TextMeasurer } from "../../../../browser/config/fontMeasurements.js";
import { EditorSelectionController } from "../../../../common/cursor/editorSelectionController.js";
import { TextSelection, TextSelectionSet } from "../../../../common/core/selection.js";
import { TextPosition } from "../../../../common/core/text.js";
import { TextModel } from "../../../../common/model/textModel.js";
import { type ClipboardControllerOptions } from "../../browser/clipboardController.js";

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

class MemoryClipboardData {
	files: readonly File[] = [];
	private readonly values = new Map<string, string>();

	get types(): readonly string[] {
		return [...this.values.keys()];
	}

	getData(type: string): string {
		return this.values.get(type) ?? "";
	}

	setData(type: string, value: string): void {
		this.values.set(type, value);
	}
}

class DeferredTextFile {
	private readonly result: Promise<string>;
	private resolveResult: ((text: string) => void) | undefined;

	constructor(readonly name: string, readonly type = "", readonly size = 16) {
		this.result = new Promise(resolve => {
			this.resolveResult = resolve;
		});
	}

	text(): Promise<string> {
		return this.result;
	}

	resolve(text: string): void {
		this.resolveResult?.(text);
	}
}

class DeferredSystemTextReader {
	private readonly requests: Array<(text: string) => void> = [];

	get requestCount(): number {
		return this.requests.length;
	}

	readText(): Promise<string> {
		return new Promise(resolve => this.requests.push(resolve));
	}

	resolve(requestIndex: number, text: string): void {
		this.requests[requestIndex]?.(text);
	}
}

class DeferredRichTextWriter {
	private readonly requests: Array<{ readonly item: { readonly plainText: string; readonly html: string }; readonly resolve: () => void; readonly reject: (error: Error) => void }> = [];

	get requestCount(): number {
		return this.requests.length;
	}

	get item(): { readonly plainText: string; readonly html: string } | undefined {
		return this.requests[0]?.item;
	}

	writeText(item: { readonly plainText: string; readonly html: string }): Promise<void> {
		return new Promise((resolve, reject) => this.requests.push({ item, resolve, reject }));
	}

	resolve(requestIndex = 0): void {
		this.requests[requestIndex]?.resolve();
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
	InputEvent: browserEnvironment.window.InputEvent,
	KeyboardEvent: browserEnvironment.window.KeyboardEvent,
})) {
	Object.defineProperty(globalThis, name, {
		configurable: true,
		value,
	});
}

const { EditorViewport } = await import("../../../../browser/view/editorViewport.js");
const { ClipboardController, EDITOR_CLIPBOARD_MIME, EDITOR_HTML_CLIPBOARD_MIME, ClipboardLineEnding } = await import("../../browser/clipboardController.js");
const { UriListPasteProvider } = await import("../../browser/clipboardPasteProvider.js");
const { EditorClipboardPasteMode, EditorEmptySelectionClipboardPolicy } = await import("../../common/clipboard.js");
const { SemanticTokenPresentation } = await import("../../../../browser/viewparts/semanticTokens/semanticTokenPresentation.js");
const { EditorView } = await import("../../../../browser/view.js");

function attachClipboard(
	input: InstanceType<typeof EditorView>,
	viewport: InstanceType<typeof EditorViewport>,
	selections: EditorSelectionController,
	options: ClipboardControllerOptions = {},
): InstanceType<typeof ClipboardController> {
	return new ClipboardController(input.editContext, viewport, selections, {
		...options,
		isEditingAllowed: () => !input.compositionController.composing && (options.isEditingAllowed?.() ?? true),
		pasteProviders: [UriListPasteProvider, ...(options.pasteProviders ?? [])],
	});
}

test("Clipboard copies, distributes paste, cuts, and restores isolated history", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector("main");
	assert.ok(container);
	using model = new TextModel("one two\nthree four");
	const copiedSelections = TextSelectionSet.withPrimary([
		selection(0, 0, 0, 3),
		selection(1, 0, 1, 5),
	], 1);
	using selections = new EditorSelectionController(model, copiedSelections);
	using viewport = new EditorViewport({
		container,
		model,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
		selectionController: selections,
	});
	viewport.layout({ width: 80, height: 40 });
	using input = new EditorView(viewport, selections);
	using clipboard = attachClipboard(input, viewport, selections, {
		lineEnding: ClipboardLineEnding.LF,
	});

	const copiedData = new MemoryClipboardData();
	const copy = clipboardEvent(dom.window, "copy", copiedData);
	input.element.dispatchEvent(copy);
	assert.equal(copy.defaultPrevented, true);
	assert.equal(copiedData.getData("text/plain"), "one\nthree");
	assert.equal(copiedData.getData(EDITOR_HTML_CLIPBOARD_MIME), "<pre><code>one\nthree</code></pre>");
	assert.deepEqual(
		JSON.parse(copiedData.getData(EDITOR_CLIPBOARD_MIME)),
		{
			version: 2,
			selectionTexts: ["one", "three"],
			pasteModes: [
				EditorClipboardPasteMode.Selection,
				EditorClipboardPasteMode.Selection,
			],
		},
	);

	const pasteTargets = TextSelectionSet.withPrimary([
		selection(0, 4, 0, 7),
		selection(1, 6, 1, 10),
	], 1);
	selections.setSelections(pasteTargets);
	const paste = clipboardEvent(dom.window, "paste", copiedData);
	input.element.dispatchEvent(paste);
	assert.equal(paste.defaultPrevented, true);
	assert.deepEqual({
		text: model.getText(),
		selections: selections.selections,
	}, {
		text: "one one\nthree three",
		selections: TextSelectionSet.withPrimary([
			caret(0, 7),
			caret(1, 11),
		], 1),
	});

	selections.undo();
	assert.deepEqual({
		text: model.getText(),
		selections: selections.selections,
	}, {
		text: "one two\nthree four",
		selections: pasteTargets,
	});

	const cutData = new MemoryClipboardData();
	const cut = clipboardEvent(dom.window, "cut", cutData);
	input.element.dispatchEvent(cut);
	assert.equal(cut.defaultPrevented, true);
	assert.equal(cutData.getData("text/plain"), "two\nfour");
	assert.equal(model.getText(), "one \nthree ");
	selections.undo();
	assert.equal(model.getText(), "one two\nthree four");

	dom.window.close();
});

test("Clipboard repeats external text and copies an empty selection as a line", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector("main");
	assert.ok(container);
	using model = new TextModel("a b");
	using selections = new EditorSelectionController(
		model,
		TextSelectionSet.withPrimary([caret(0, 0), caret(0, 2)], 0),
	);
	using viewport = new EditorViewport({
		container,
		model,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
		selectionController: selections,
	});
	viewport.layout({ width: 80, height: 20 });
	const input = new EditorView(viewport, selections);
	const clipboard = attachClipboard(input, viewport, selections, {
		lineEnding: ClipboardLineEnding.LF,
	});

	const externalData = new MemoryClipboardData();
	externalData.setData("text/plain", "X\r\nY");
	externalData.setData(EDITOR_CLIPBOARD_MIME, JSON.stringify({
		version: 1,
		selectionTexts: ["wrong count"],
	}));
	const paste = clipboardEvent(dom.window, "paste", externalData);
	input.element.dispatchEvent(paste);
	assert.equal(paste.defaultPrevented, true);
	assert.deepEqual({
		text: model.getText(),
		selections: selections.selections,
	}, {
		text: "X\nYa X\nYb",
		selections: TextSelectionSet.withPrimary([
			caret(1, 1),
			caret(2, 1),
		], 0),
	});

	selections.setSelections(TextSelectionSet.single(caret(0, 0)));
	const emptyData = new MemoryClipboardData();
	const emptyCopy = clipboardEvent(dom.window, "copy", emptyData);
	input.element.dispatchEvent(emptyCopy);
	assert.equal(emptyCopy.defaultPrevented, true);
	assert.equal(emptyData.getData("text/plain"), "X\n");

	clipboard.dispose();
	input.dispose();
	const disposedData = new MemoryClipboardData();
	disposedData.setData("text/plain", "ignored");
	const disposedPaste = clipboardEvent(dom.window, "paste", disposedData);
	input.element.dispatchEvent(disposedPaste);
	assert.equal(disposedPaste.defaultPrevented, false);
	assert.equal(model.getText(), "X\nYa X\nYb");

	dom.window.close();
});

test("Clipboard round-trips complete lines and preserves target columns", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector("main");
	assert.ok(container);
	using model = new TextModel("one\ntwo\nthree");
	using selections = new EditorSelectionController(
		model,
		TextSelectionSet.withPrimary([caret(0, 1), caret(2, 2)], 1),
	);
	using viewport = new EditorViewport({
		container,
		model,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
		selectionController: selections,
	});
	viewport.layout({ width: 80, height: 40 });
	using input = new EditorView(viewport, selections);
	using clipboard = attachClipboard(input, viewport, selections, {
		lineEnding: ClipboardLineEnding.LF,
	});

	const lineData = new MemoryClipboardData();
	input.element.dispatchEvent(clipboardEvent(dom.window, "copy", lineData));
	assert.equal(lineData.getData("text/plain"), "one\nthree\n");
	assert.deepEqual(
		JSON.parse(lineData.getData(EDITOR_CLIPBOARD_MIME)),
		{
			version: 2,
			selectionTexts: ["one\n", "three\n"],
			pasteModes: [
				EditorClipboardPasteMode.Line,
				EditorClipboardPasteMode.Line,
			],
		},
	);

	const targets = TextSelectionSet.withPrimary([
		caret(0, 2),
		caret(1, 1),
	], 1);
	selections.setSelections(targets);
	input.element.dispatchEvent(clipboardEvent(dom.window, "paste", lineData));
	assert.deepEqual({
		text: model.getText(),
		selections: selections.selections,
	}, {
		text: "one\none\nthree\ntwo\nthree",
		selections: TextSelectionSet.withPrimary([
			caret(1, 2),
			caret(3, 1),
		], 1),
	});

	selections.undo();
	assert.deepEqual({
		text: model.getText(),
		selections: selections.selections,
	}, {
		text: "one\ntwo\nthree",
		selections: targets,
	});

	selections.setSelections(TextSelectionSet.single(caret(1, 2)));
	const cutData = new MemoryClipboardData();
	input.element.dispatchEvent(clipboardEvent(dom.window, "cut", cutData));
	assert.deepEqual({
		clipboard: cutData.getData("text/plain"),
		text: model.getText(),
		selection: selections.selections.primary,
	}, {
		clipboard: "two\n",
		text: "one\nthree",
		selection: caret(1, 0),
	});
	selections.undo();
	assert.equal(model.getText(), "one\ntwo\nthree");

	dom.window.close();
});

test("Mixed line and selection metadata falls back to selection paste", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector("main");
	assert.ok(container);
	using model = new TextModel("a\nb");
	using selections = new EditorSelectionController(
		model,
		TextSelectionSet.withPrimary([
			caret(0, 1),
			selection(1, 0, 1, 1),
		], 1),
	);
	using viewport = new EditorViewport({
		container,
		model,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
		selectionController: selections,
	});
	using input = new EditorView(viewport, selections);
	using clipboard = attachClipboard(input, viewport, selections, {
		lineEnding: ClipboardLineEnding.LF,
	});

	const data = new MemoryClipboardData();
	input.element.dispatchEvent(clipboardEvent(dom.window, "copy", data));
	assert.deepEqual(
		JSON.parse(data.getData(EDITOR_CLIPBOARD_MIME)).pasteModes,
		[EditorClipboardPasteMode.Line, EditorClipboardPasteMode.Selection],
	);

	selections.setSelections(TextSelectionSet.withPrimary([
		caret(0, 0),
		caret(1, 0),
	], 1));
	input.element.dispatchEvent(clipboardEvent(dom.window, "paste", data));
	assert.deepEqual({
		text: model.getText(),
		selections: selections.selections,
	}, {
		text: "a\na\nbb",
		selections: TextSelectionSet.withPrimary([
			caret(1, 0),
			caret(2, 1),
		], 1),
	});

	dom.window.close();
});

test("Empty-selection clipboard policy may explicitly preserve browser behavior", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector("main");
	assert.ok(container);
	using model = new TextModel("abc");
	using selections = new EditorSelectionController(
		model,
		TextSelectionSet.single(caret(0, 1)),
	);
	using viewport = new EditorViewport({
		container,
		model,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
		selectionController: selections,
	});
	using input = new EditorView(viewport, selections);
	using clipboard = attachClipboard(input, viewport, selections, {
		lineEnding: ClipboardLineEnding.LF,
		emptySelectionPolicy: EditorEmptySelectionClipboardPolicy.Ignore,
	});

	const data = new MemoryClipboardData();
	const copy = clipboardEvent(dom.window, "copy", data);
	input.element.dispatchEvent(copy);
	assert.equal(copy.defaultPrevented, false);
	assert.equal(data.getData("text/plain"), "");

	dom.window.close();
});

test("Clipboard copies escaped HTML and safely falls back to external HTML text", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector("main");
	assert.ok(container);
	using model = new TextModel("if (a < b && c > d) {}");
	using selections = new EditorSelectionController(model, TextSelectionSet.single(selection(0, 0, 0, model.getLineContent(0).length)));
	using viewport = new EditorViewport({
		container,
		model,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
		selectionController: selections,
	});
	using input = new EditorView(viewport, selections);
	using clipboard = attachClipboard(input, viewport, selections);

	const copied = new MemoryClipboardData();
	input.element.dispatchEvent(clipboardEvent(dom.window, "copy", copied));
	assert.equal(copied.getData(EDITOR_HTML_CLIPBOARD_MIME), "<pre><code>if (a &lt; b &amp;&amp; c &gt; d) {}</code></pre>");

	selections.setSelections(TextSelectionSet.single(caret(0, model.getLineContent(0).length)));
	const external = new MemoryClipboardData();
	external.setData(EDITOR_HTML_CLIPBOARD_MIME, "<div>first &amp; second</div><div><strong>third</strong><br>fourth</div><script>ignored()</script>");
	const paste = clipboardEvent(dom.window, "paste", external);
	input.element.dispatchEvent(paste);
	assert.equal(paste.defaultPrevented, true);
	assert.equal(model.getText(), "if (a < b && c > d) {}first & second\nthird\nfourth");

	dom.window.close();
});

test("Clipboard preserves current semantic token markup in portable HTML", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector("main");
	assert.ok(container);
	dom.window.document.documentElement.style.setProperty("--zeta-editor-token-keyword-foreground", "rgb(1, 2, 3)");
	using model = new TextModel("const value\nnext");
	using selections = new EditorSelectionController(model, TextSelectionSet.single(selection(0, 0, 1, model.getLineContent(1).length)));
	using viewport = new EditorViewport({
		container,
		model,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
		selectionController: selections,
	});
	const semanticTokens = {
		textModel: model,
		onDidChange: () => ({ dispose() {}, [Symbol.dispose]() {} }),
		lines: [],
		getLineTokens: (lineIndex: number) => lineIndex === 0
			? [{
				startColumn: 0,
				endColumn: 5,
				presentation: SemanticTokenPresentation.Keyword,
			}]
			: lineIndex === 1
				? [{
					startColumn: 0,
					endColumn: 4,
					presentation: SemanticTokenPresentation.Keyword,
				}]
				: [],
	};
	using input = new EditorView(viewport, selections);
	using clipboard = attachClipboard(input, viewport, selections, {
		lineEnding: ClipboardLineEnding.LF,
		semanticTokens,
	});

	const copied = new MemoryClipboardData();
	input.element.dispatchEvent(clipboardEvent(dom.window, "copy", copied));
	assert.equal(copied.getData("text/plain"), "const value\nnext");
	assert.equal(
		copied.getData(EDITOR_HTML_CLIPBOARD_MIME),
		'<pre><code><span class="stanza-editor-token token-keyword" style="color: rgb(1, 2, 3)">const</span> value\n<span class="stanza-editor-token token-keyword" style="color: rgb(1, 2, 3)">next</span></code></pre>',
	);

	selections.setSelections(TextSelectionSet.single(caret(1, 2)));
	const lineCopied = new MemoryClipboardData();
	input.element.dispatchEvent(clipboardEvent(dom.window, "copy", lineCopied));
	assert.equal(
		lineCopied.getData(EDITOR_HTML_CLIPBOARD_MIME),
		'<pre><code><span class="stanza-editor-token token-keyword" style="color: rgb(1, 2, 3)">next</span>\n</code></pre>',
	);

	dom.window.close();
});

test("Clipboard reads one user-provided text file only while its revision and selections remain current", async () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector("main");
	assert.ok(container);
	using model = new TextModel("one");
	using selections = new EditorSelectionController(model, TextSelectionSet.single(caret(0, 3)));
	using viewport = new EditorViewport({
		container,
		model,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
		selectionController: selections,
	});
	using input = new EditorView(viewport, selections);
	using clipboard = attachClipboard(input, viewport, selections);
	const file = new DeferredTextFile("snippet.rs");
	const data = new MemoryClipboardData();
	data.files = [file as unknown as File];

	const paste = clipboardEvent(dom.window, "paste", data);
	input.element.dispatchEvent(paste);
	assert.equal(paste.defaultPrevented, true);
	file.resolve(" two\r\nthree");
	await flushPromises();
	assert.equal(model.getText(), "one two\nthree");

	const staleFile = new DeferredTextFile("later.ts", "text/plain");
	const staleData = new MemoryClipboardData();
	staleData.files = [staleFile as unknown as File];
	input.element.dispatchEvent(clipboardEvent(dom.window, "paste", staleData));
	selections.setSelections(TextSelectionSet.single(caret(0, 0)));
	staleFile.resolve("ignored");
	await flushPromises();
	assert.equal(model.getText(), "one two\nthree");

	dom.window.close();
});

test("Clipboard runs local URI providers and discards stale asynchronous provider results", async () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector("main");
	assert.ok(container);
	using model = new TextModel("one");
	using selections = new EditorSelectionController(model, TextSelectionSet.single(caret(0, 3)));
	using viewport = new EditorViewport({
		container,
		model,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
		selectionController: selections,
	});
	let resolveProvidedText: ((text: string) => void) | undefined;
	const providedText = new Promise<string>(resolve => {
		resolveProvidedText = resolve;
	});
	using input = new EditorView(viewport, selections);
	using clipboard = attachClipboard(input, viewport, selections, {
		pasteProviders: [{
			id: "test.delayed-snippet",
			mimeTypes: ["application/x-zeta-snippet"],
			providePaste: () => providedText,
		}],
	});

	const uriData = new MemoryClipboardData();
	uriData.setData("text/uri-list", "# copied URI\nfile:///workspace/one.rs\nhttps://example.test/two");
	const uriPaste = clipboardEvent(dom.window, "paste", uriData);
	input.element.dispatchEvent(uriPaste);
	assert.equal(uriPaste.defaultPrevented, true);
	await flushPromises();
	assert.equal(model.getText(), "onefile:///workspace/one.rs\nhttps://example.test/two");

	selections.setSelections(TextSelectionSet.single(caret(0, 0)));
	const delayedData = new MemoryClipboardData();
	delayedData.setData("application/x-zeta-snippet", "opaque");
	const delayedPaste = clipboardEvent(dom.window, "paste", delayedData);
	input.element.dispatchEvent(delayedPaste);
	assert.equal(delayedPaste.defaultPrevented, true);
	selections.setSelections(TextSelectionSet.single(caret(0, 1)));
	resolveProvidedText?.("must not apply");
	await flushPromises();
	assert.equal(model.getText(), "onefile:///workspace/one.rs\nhttps://example.test/two");

	dom.window.close();
});

test("Clipboard uses the system text reader only as a stale-safe empty-transfer fallback", async () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector("main");
	assert.ok(container);
	using model = new TextModel("one");
	using selections = new EditorSelectionController(model, TextSelectionSet.single(caret(0, 3)));
	using viewport = new EditorViewport({
		container,
		model,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
		selectionController: selections,
	});
	const systemTextReader = new DeferredSystemTextReader();
	using input = new EditorView(viewport, selections);
	using clipboard = attachClipboard(input, viewport, selections, { systemTextReader });

	const fallbackPaste = clipboardEvent(dom.window, "paste", new MemoryClipboardData());
	input.element.dispatchEvent(fallbackPaste);
	assert.equal(fallbackPaste.defaultPrevented, true);
	assert.equal(systemTextReader.requestCount, 1);
	systemTextReader.resolve(0, " two");
	await flushPromises();
	assert.equal(model.getText(), "one two");

	selections.setSelections(TextSelectionSet.single(caret(0, 0)));
	input.element.dispatchEvent(clipboardEvent(dom.window, "paste", new MemoryClipboardData()));
	assert.equal(systemTextReader.requestCount, 2);
	selections.setSelections(TextSelectionSet.single(caret(0, 1)));
	systemTextReader.resolve(1, "ignored");
	await flushPromises();
	assert.equal(model.getText(), "one two");

	const nativeText = new MemoryClipboardData();
	nativeText.setData("text/plain", "!");
	selections.setSelections(TextSelectionSet.single(caret(0, model.getLineContent(0).length)));
	input.element.dispatchEvent(clipboardEvent(dom.window, "paste", nativeText));
	assert.equal(systemTextReader.requestCount, 2);
	assert.equal(model.getText(), "one two!");

	dom.window.close();
});

test("Clipboard safely prefers the rich system reader before its plain-text fallback", async () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector("main");
	assert.ok(container);
	using model = new TextModel("one");
	using selections = new EditorSelectionController(model, TextSelectionSet.single(caret(0, 3)));
	using viewport = new EditorViewport({ container, model, lineHeight: 20, textMeasurer: new FixedTextMeasurer(), selectionController: selections });
	let plainReaderCalls = 0;
	using input = new EditorView(viewport, selections);
	using clipboard = attachClipboard(input, viewport, selections, {
		richTextReader: { readText: () => Promise.resolve({ html: "<b> two</b><script>ignored()</script>" }) },
		systemTextReader: { readText: () => { plainReaderCalls += 1; return Promise.resolve(" fallback"); } },
	});
	const paste = clipboardEvent(dom.window, "paste", new MemoryClipboardData());
	input.element.dispatchEvent(paste);
	assert.equal(paste.defaultPrevented, true);
	await flushPromises();
	assert.equal(model.getText(), "one two");
	assert.equal(plainReaderCalls, 0);
	dom.window.close();
});

test("Clipboard falls back to Async rich copy and delays cut until it succeeds", async () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector("main");
	assert.ok(container);
	using model = new TextModel("one");
	using selections = new EditorSelectionController(model, TextSelectionSet.single(selection(0, 0, 0, 3)));
	using viewport = new EditorViewport({ container, model, lineHeight: 20, textMeasurer: new FixedTextMeasurer(), selectionController: selections });
	const writer = new DeferredRichTextWriter();
	using input = new EditorView(viewport, selections);
	using clipboard = attachClipboard(input, viewport, selections, { richTextWriter: writer });

	const copy = clipboardEvent(dom.window, "copy", null);
	input.element.dispatchEvent(copy);
	assert.equal(copy.defaultPrevented, true);
	assert.equal(writer.requestCount, 1);
	assert.deepEqual(writer.item, { plainText: "one", html: "<pre><code>one</code></pre>" });
	writer.resolve();
	await flushPromises();
	assert.equal(model.getText(), "one");

	const cut = clipboardEvent(dom.window, "cut", null);
	input.element.dispatchEvent(cut);
	assert.equal(cut.defaultPrevented, true);
	assert.equal(writer.requestCount, 2);
	assert.equal(model.getText(), "one");
	writer.resolve(1);
	await flushPromises();
	assert.equal(model.getText(), "");
	dom.window.close();
});

test("Clipboard preserves an active IME composition by rejecting mutable clipboard operations", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const container = dom.window.document.querySelector("main");
	assert.ok(container);
	using model = new TextModel("one");
	using selections = new EditorSelectionController(model, TextSelectionSet.single(caret(0, 3)));
	using viewport = new EditorViewport({
		container,
		model,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
		selectionController: selections,
	});
	using input = new EditorView(viewport, selections);
	using clipboard = attachClipboard(input, viewport, selections);
	input.element.dispatchEvent(compositionEvent(dom.window, "compositionstart", ""));
	assert.equal(input.compositionController.composing, true);

	const pasteData = new MemoryClipboardData();
	pasteData.setData("text/plain", " changed");
	const paste = clipboardEvent(dom.window, "paste", pasteData);
	input.element.dispatchEvent(paste);
	assert.equal(paste.defaultPrevented, true);
	assert.equal(model.getText(), "one");
	assert.equal(input.compositionController.composing, true);

	input.element.dispatchEvent(compositionEvent(dom.window, "compositionend", ""));
	assert.equal(input.compositionController.composing, false);
	dom.window.close();
});

function clipboardEvent(targetWindow: typeof browserEnvironment.window, type: "copy" | "cut" | "paste", clipboardData: MemoryClipboardData | null): ClipboardEvent {
	const event = new targetWindow.Event(type, {
		bubbles: true,
		cancelable: true,
	}) as unknown as ClipboardEvent;
	Object.defineProperty(event, "clipboardData", {
		configurable: true,
		value: clipboardData as unknown as DataTransfer | null,
	});
	return event;
}

function compositionEvent(targetWindow: typeof browserEnvironment.window, type: "compositionstart" | "compositionend", data: string): CompositionEvent {
	const event = new targetWindow.Event(type, {
		bubbles: true,
		cancelable: true,
	}) as unknown as CompositionEvent;
	Object.defineProperty(event, "data", {
		configurable: true,
		value: data,
	});
	return event;
}

function selection(startLine: number, startColumn: number, endLine: number, endColumn: number): TextSelection {
	return TextSelection.from(
		TextPosition.at(startLine, startColumn),
		TextPosition.at(endLine, endColumn),
	);
}

function caret(lineIndex: number, columnIndex: number): TextSelection {
	return TextSelection.collapsedAt(TextPosition.at(lineIndex, columnIndex));
}

async function flushPromises(): Promise<void> {
	await Promise.resolve();
	await Promise.resolve();
}
