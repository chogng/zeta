import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { URI } from "../../../../../base/common/uri.js";
import { TextPosition, TextRange } from "../../../../../editor/common/core/text.js";
import { EditorPaneVisibility } from "../../../../browser/parts/editor/editorPane.js";
import { TextFileContentSource, type ITextFileService, type ResolvedTextFileContent, type TextFileResolveRequest } from "../../../../services/textfile/common/textFileService.js";
import { LanguageFeaturesService } from "../../../../services/language/common/languageFeaturesService.js";
import { toDisposable } from "../../../../../base/common/lifecycle.js";
import { type ILanguageDiagnosticsService, type LanguageDiagnosticsPublisher, type LanguageDiagnosticSnapshot } from "../../../../../editor/common/services/languageDiagnosticsService.js";
import { type TextModel } from "../../../../../editor/common/model/textModel.js";
import type { EditorPanePartOptions } from "../../browser/codeEditorPane.js";

const browserEnvironment = new JSDOM("<!doctype html><body></body>");
for (const [name, value] of Object.entries({
	window: browserEnvironment.window,
	document: browserEnvironment.window.document,
	Node: browserEnvironment.window.Node,
	Element: browserEnvironment.window.Element,
	HTMLElement: browserEnvironment.window.HTMLElement,
	Event: browserEnvironment.window.Event,
	InputEvent: browserEnvironment.window.InputEvent,
})) {
	Object.defineProperty(globalThis, name, { configurable: true, value });
}

await import("../../../../../editor/editor.code.all.js");
const { CodeEditorPane: EditorPane } = await import("../../browser/codeEditorPane.js");
const { BrowserTextModelService } = await import("../../../../../editor/browser/services/browserTextModelService.js");
const { BrowserTextResourceStore } = await import("../../browser/browserTextResourceStore.js");
const { EditorTextDirection } = await import("../../../../../editor/browser/view/editorViewport.js");
const { EditorMinimap } = await import("../../../../../editor/browser/view/editorViewport.js");
const { EditorIndentationKind } = await import("../../../../../editor/common/editorIndentation.js");
const { EditorLineWrapping } = await import("../../../../../editor/browser/viewModel/visualLineProjection.js");

test.after(() => browserEnvironment.window.close());

test("Stanza editor pane loads, lays out, focuses, hides, and clears one editor part", async () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	const parent = dom.window.document.querySelector<HTMLElement>("main")!;
	const textFiles = new ImmediateTextFiles("from disk");
	const resourceStore = new BrowserTextResourceStore(textFiles);
	using models = new BrowserTextModelService(resourceStore);
	const pane = new EditorPane(resourceStore, { modelService: models, textDirection: EditorTextDirection.RightToLeft, fontFamily: "Fira Code, monospace", fontSize: 16 });
	pane.create(parent);
	pane.layout({ width: 640, height: 480 });
	await pane.setInput({
		resource: URI.file("C:\\project\\main.ts"),
		label: "main.ts",
		initialText: "const alpha = 1;",
	}, new AbortController().signal);

	assert.equal(pane.getValue(), "const alpha = 1;");
	assert.equal(parent.querySelectorAll(".stanza-editor-pane").length, 1);
	assert.equal(parent.querySelectorAll(".stanza-editor").length, 1);
	const editor = parent.querySelector<HTMLElement>(".stanza-editor")!;
	assert.equal(editor.dir, "rtl");
	assert.equal(editor.style.fontFamily, '"Fira Code", monospace');
	assert.equal(editor.style.fontSize, "16px");
	pane.focus();
	assert.equal(dom.window.document.activeElement?.classList.contains("stanza-editor-input"), true);
	assert.equal((dom.window.document.activeElement as HTMLTextAreaElement).dir, "rtl");
	pane.setVisible(EditorPaneVisibility.Hidden);
	assert.equal((parent.firstElementChild as HTMLElement).hidden, true);
	pane.setVisible(EditorPaneVisibility.Visible);
	assert.equal((parent.firstElementChild as HTMLElement).hidden, false);

	pane.clearInput();
	assert.equal(pane.getValue(), "");
	assert.equal(parent.querySelectorAll(".stanza-editor").length, 0);
	pane.dispose();
	assert.equal(parent.children.length, 0);
	dom.window.close();
});

test("Stanza editor pane acquires the Workbench language service for its detected model", async () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	const parent = dom.window.document.querySelector<HTMLElement>("main")!;
	const textFiles = new ImmediateTextFiles("const value = 1;");
	const resourceStore = new BrowserTextResourceStore(textFiles);
	using models = new BrowserTextModelService(resourceStore);
	using languages = new LanguageFeaturesService();
	const diagnostics = new RecordingLanguageDiagnosticsService();
	const pane = new EditorPane(resourceStore, { modelService: models, languageFeaturesService: languages, languageDiagnosticsService: diagnostics });
	pane.create(parent);
	const resource = URI.file("C:\\project\\main.ts");

	await pane.setInput({ resource }, new AbortController().signal);

	assert.deepEqual(diagnostics.acquired.map(entry => ({ resource: entry.resource.toString(), languageId: entry.languageId })), [{ resource: resource.toString(), languageId: "typescript" }]);
	assert.equal(diagnostics.activeAcquisitions, 1);
	pane.clearInput();
	assert.equal(diagnostics.activeAcquisitions, 0);
	pane.dispose();
	dom.window.close();
});

class RecordingLanguageDiagnosticsService implements ILanguageDiagnosticsService {
	readonly acquired: Array<{ readonly resource: URI; readonly languageId: string; readonly model: TextModel }> = [];
	activeAcquisitions = 0;
	readonly onDidChangeDiagnostics = () => toDisposable(() => undefined);
	acquire(resource: URI, languageId: string, model: TextModel) {
		this.acquired.push({ resource, languageId, model });
		this.activeAcquisitions += 1;
		return toDisposable(() => { this.activeAcquisitions -= 1; });
	}
	createPublisher(): LanguageDiagnosticsPublisher { return { update: () => undefined, dispose: () => undefined, [Symbol.dispose]: () => undefined }; }
	getDiagnostics(): LanguageDiagnosticSnapshot | undefined { return undefined; }
	getAllDiagnostics(): readonly LanguageDiagnosticSnapshot[] { return []; }
}

test("Stanza editor pane releases a load cancelled before content resolution", async () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	const parent = dom.window.document.querySelector<HTMLElement>("main")!;
	const pending = deferred<ResolvedTextFileContent>();
	const textFiles = { onDidChangeFiles: inertFileChanges, resolve: () => pending.promise, save: async () => ({ revision: undefined }) };
	const resourceStore = new BrowserTextResourceStore(textFiles);
	using models = new BrowserTextModelService(resourceStore);
	const pane = new EditorPane(resourceStore, { modelService: models });
	pane.create(parent);
	const controller = new AbortController();
	const opening = pane.setInput({ resource: URI.file("C:\\project\\slow.ts") }, controller.signal);
	controller.abort();
	pending.resolve({
		resource: URI.file("C:\\project\\slow.ts"),
		text: "late",
		source: TextFileContentSource.FileSystem,
		revision: "revision-1",
		encoding: "utf8",
	});

	await assert.rejects(opening, error => (error as Error).name === "CancellationError");
	assert.equal(parent.querySelectorAll(".stanza-editor").length, 0);
	pane.dispose();
	dom.window.close();
});

test("Stanza editor pane saves and reverts its shared model reference", async () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	const parent = dom.window.document.querySelector<HTMLElement>("main")!;
	const textFiles = new ImmediateTextFiles("from disk");
	const resourceStore = new BrowserTextResourceStore(textFiles);
	using models = new BrowserTextModelService(resourceStore);
	const resource = URI.file("C:\\project\\main.ts");
	const reference = await models.acquire({ resource }, new AbortController().signal);
	const pane = new EditorPane(resourceStore, { modelService: models });
	pane.create(parent);
	await pane.setInput({ resource, label: "main.ts" }, new AbortController().signal);

	reference.model.applyEdits([{
		range: TextRange.from(TextPosition.at(0, 0), TextPosition.at(0, 4)),
		text: "saved",
	}]);
	assert.equal(pane.isDirty, true);
	await pane.save();
	assert.deepEqual(textFiles.savedTexts, ["saved disk"]);
	assert.equal(pane.isDirty, false);

	reference.model.applyEdits([{
		range: TextRange.emptyAt(TextPosition.at(0, 5)),
		text: " locally",
	}]);
	textFiles.setText("from disk");
	await pane.revert();
	assert.equal(pane.getValue(), "from disk");
	assert.equal(pane.isDirty, false);

	reference.dispose();
	pane.dispose();
	dom.window.close();
});

test("Stanza editor pane resolves extension first-line languages after loading an unknown file", async () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const parent = dom.window.document.querySelector<HTMLElement>("main")!;
	const textFiles = new ImmediateTextFiles("#!/usr/bin/env demo\nprint('ok')");
	const resourceStore = new BrowserTextResourceStore(textFiles);
	using models = new BrowserTextModelService(resourceStore);
	using languages = new LanguageFeaturesService();
	using registration = languages.registerLanguage({ id: "demo", firstLine: "#!.*\\bdemo" }, { priority: 100 });
	let languageId: string | undefined;
	const pane = new EditorPane(resourceStore, {
		modelService: models,
		languageFeaturesService: languages,
		createPart: options => {
			languageId = options.languageId;
			return { layout: () => {}, focus: () => {}, getValue: () => "", dispose: () => {}, [Symbol.dispose]: () => {} };
		},
	});
	pane.create(parent);

	await pane.setInput({ resource: URI.file("C:\\project\\script.cgi") }, new AbortController().signal);

	assert.equal(languageId, "demo");
	pane.dispose();
	dom.window.close();
});

test("Stanza editor pane forwards Workbench editor preferences to each created part", async () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const parent = dom.window.document.querySelector<HTMLElement>("main")!;
	const textFiles = new ImmediateTextFiles("const value = 1;");
	const resourceStore = new BrowserTextResourceStore(textFiles);
	using models = new BrowserTextModelService(resourceStore);
	let received: EditorPanePartOptions | undefined;
	const pane = new EditorPane(resourceStore, {
		modelService: models,
		fontFamily: "Fira Code, monospace",
		fontSize: 16,
		lineHeight: 26,
		fontLigatures: true,
		lineWrapping: EditorLineWrapping.On,
		minimap: EditorMinimap.Off,
		activeLineHighlight: "off",
		showLineNumbers: false,
		showIndentationGuides: false,
		bracketPairColorization: false,
		stickyScroll: false,
		suggestions: false,
		inlineCompletions: false,
		parameterHints: false,
		inlayHints: false,
		codeLens: false,
		formatOnSave: true,
		find: {
			seedSearchStringFromSelection: false,
			autoFindInSelection: true,
			loop: false,
			matchCase: true,
			wholeWord: true,
			regularExpression: true,
		},
		indentation: { kind: EditorIndentationKind.Tabs, tabSize: 2 },
		showUnicodeHighlights: false,
		insertFinalNewLine: true,
		createPart: options => {
			received = options;
			return { layout: () => {}, focus: () => {}, getValue: () => "", dispose: () => {}, [Symbol.dispose]: () => {} };
		},
	});
	pane.create(parent);
	await pane.setInput({ resource: URI.file("C:\\project\\configured.ts") }, new AbortController().signal);

	assert.equal(received?.lineWrapping, EditorLineWrapping.On);
	assert.equal(received?.fontFamily, "Fira Code, monospace");
	assert.equal(received?.fontSize, 16);
	assert.equal(received?.lineHeight, 26);
	assert.equal(received?.fontLigatures, true);
	assert.equal(received?.minimap, EditorMinimap.Off);
	assert.equal(received?.activeLineHighlight, "off");
	assert.equal(received?.showLineNumbers, false);
	assert.equal(received?.showIndentationGuides, false);
	assert.equal(received?.bracketPairColorization, false);
	assert.equal(received?.stickyScroll, false);
	assert.equal(received?.suggestions, false);
	assert.equal(received?.inlineCompletions, false);
	assert.equal(received?.parameterHints, false);
	assert.equal(received?.inlayHints, false);
	assert.equal(received?.codeLens, false);
	assert.equal(received?.formatOnSave, true);
	assert.deepEqual(received?.find, {
		seedSearchStringFromSelection: false,
		autoFindInSelection: true,
		loop: false,
		matchCase: true,
		wholeWord: true,
		regularExpression: true,
	});
	assert.deepEqual(received?.indentation, { kind: EditorIndentationKind.Tabs, tabSize: 2 });
	assert.equal(received?.showUnicodeHighlights, false);
	assert.equal(received?.insertFinalNewLine, true);
	pane.dispose();
	dom.window.close();
});

test("Workbench owns the code editor save shortcut and reports failures", async () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	const parent = dom.window.document.querySelector<HTMLElement>("main")!;
	const textFiles = new ImmediateTextFiles("alpha");
	const resourceStore = new BrowserTextResourceStore(textFiles);
	using models = new BrowserTextModelService(resourceStore);
	const errors: unknown[] = [];
	const pane = new EditorPane(resourceStore, { modelService: models, onSaveError: error => errors.push(error) });
	pane.create(parent);
	await pane.setInput({ resource: URI.file("C:\\project\\save.ts") }, new AbortController().signal);

	const input = parent.querySelector<HTMLTextAreaElement>(".stanza-editor-input")!;
	input.dispatchEvent(new dom.window.KeyboardEvent("keydown", { bubbles: true, cancelable: true, ctrlKey: true, key: "s" }));
	await waitFor(() => textFiles.savedTexts.length === 1);
	assert.equal(parent.querySelector(".stanza-editor-accessibility-status")?.textContent, "Saved");

	textFiles.failSave = true;
	input.dispatchEvent(new dom.window.KeyboardEvent("keydown", { bubbles: true, cancelable: true, ctrlKey: true, key: "s" }));
	await waitFor(() => errors.length === 1);
	assert.equal(parent.querySelector(".stanza-editor-accessibility-status")?.textContent, "Save failed: conflict");

	pane.dispose();
	dom.window.close();
});

class ImmediateTextFiles implements ITextFileService {
	readonly savedTexts: string[] = [];
	readonly onDidChangeFiles = inertFileChanges;
	failSave = false;
	private revision = 1;

	constructor(private text: string) {}

	async resolve(request: TextFileResolveRequest): Promise<ResolvedTextFileContent> {
		return {
			resource: request.resource,
			text: request.bootstrapText ?? this.text,
			source: request.bootstrapText === undefined ? TextFileContentSource.FileSystem : TextFileContentSource.Bootstrap,
			revision: request.bootstrapText === undefined ? this.currentRevision() : undefined,
			encoding: "utf8",
		};
	}

	async save(request: { readonly text: string }): Promise<{ readonly revision: string | undefined }> {
		if (this.failSave) throw new Error("conflict");
		this.savedTexts.push(request.text);
		this.text = request.text;
		this.revision += 1;
		return { revision: this.currentRevision() };
	}

	setText(text: string): void {
		this.text = text;
		this.revision += 1;
	}

	private currentRevision(): string {
		return `revision-${this.revision}`;
	}
}

async function waitFor(predicate: () => boolean, timeout = 500): Promise<void> {
	const deadline = Date.now() + timeout;
	while (!predicate()) {
		if (Date.now() >= deadline) throw new Error("Timed out waiting for Workbench editor state");
		await new Promise(resolve => setTimeout(resolve, 1));
	}
}

function inertFileChanges() {
	return {
		dispose() {},
		[Symbol.dispose]() {},
	};
}

function deferred<T>(): { readonly promise: Promise<T>; readonly resolve: (value: T) => void } {
	let resolve!: (value: T) => void;
	const promise = new Promise<T>(resolver => {
		resolve = resolver;
	});
	return { promise, resolve };
}
