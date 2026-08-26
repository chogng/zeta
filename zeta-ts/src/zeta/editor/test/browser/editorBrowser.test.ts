import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { DisposableTracker, installDisposableTracker } from "../../../base/common/disposableTracker.js";
import { URI } from "../../../base/common/uri.js";
import { type TextModelReference } from "../../common/services/textModelService.js";
import { LanguageFeaturesService } from "../../common/services/languageService.js";
import { TextModel } from "../../common/model/textModel.js";
import { TextPosition, TextRange } from "../../common/core/text.js";

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

await import("../../editor.code.all.js");
const { EditorBrowser } = await import("../../browser/editorBrowser.js");

test.after(() => browserEnvironment.window.close());

test("Stanza editor browser composes native input, local language syntax, and presentation", async () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	const container = dom.window.document.querySelector<HTMLElement>("main")!;
	const model = new TextModel("{\"name\": \"alpha\"");
	const reference = modelReference(URI.file("C:\\project\\settings.json"), model);
	const errors: unknown[] = [];
	const editorPart = new EditorBrowser({
		container,
		input: {
			resource: reference.resource,
			label: "settings.json",
		},
		languageId: "json",
		modelReference: reference,
		onLanguageError: error => errors.push(error),
	});
	editorPart.layout({ width: 500, height: 240 });
	await waitFor(() => container.querySelectorAll(".stanza-editor-token.token-string").length > 0);
	await waitFor(() => container.querySelectorAll(".stanza-editor-decoration.warning-underline").length > 0);

	assert.equal(container.querySelectorAll(".stanza-editor").length, 1);
	assert.equal(container.querySelectorAll(".stanza-editor-input").length, 1);
	assert.equal(container.querySelectorAll(".stanza-editor-token.token-string").length > 0, true);
	assert.equal(container.querySelectorAll(".stanza-editor-bracket-level-1").length > 0, true);
	assert.equal(container.querySelectorAll(".stanza-editor-decoration.warning-underline").length > 0, true);
	assert.deepEqual(errors, []);

	editorPart.textInput.element.dispatchEvent(new dom.window.InputEvent("beforeinput", {
		bubbles: true,
		cancelable: true,
		data: "x",
		inputType: "insertText",
	}));
	assert.equal(editorPart.getValue().startsWith("x{"), true);

	editorPart.dispose();
	await Promise.resolve();
	await Promise.resolve();
	assert.deepEqual(errors, []);
	assert.equal(container.children.length, 0);
	assert.throws(() => model.getText(), /disposed/);
	dom.window.close();
});

test("Stanza editor browser gives language editing one disposable owner", () => {
	const tracker = new DisposableTracker();
	{
		using installation = installDisposableTracker(tracker);
		const dom = new JSDOM("<!doctype html><body><main></main></body>");
		dom.window.HTMLCanvasElement.prototype.getContext = () => null;
		const container = dom.window.document.querySelector<HTMLElement>("main")!;
		const model = new TextModel("const value = { nested: true };");
		const reference = modelReference(URI.file("C:\\project\\main.ts"), model);
		const editorPart = new EditorBrowser({ container, input: { resource: reference.resource, label: "main.ts" }, languageId: "typescript", modelReference: reference });

		editorPart.dispose();
		dom.window.close();
	}

	assert.deepEqual(tracker.leaks().filter(leak => leak.label === "LanguageEditingAdapter"), []);
});

test("Stanza editor browser derives indentation folds and projects their gutter controls", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	const container = dom.window.document.querySelector<HTMLElement>("main")!;
	const model = new TextModel("root\n  child\nafter");
	const reference = modelReference(URI.file("C:\\project\\fold.txt"), model);
	const editorPart = new EditorBrowser({
		container,
		input: {
			resource: reference.resource,
			label: "fold.txt",
		},
		languageId: "plaintext",
		modelReference: reference,
	});
	editorPart.layout({ width: 500, height: 120 });

	const foldToggle = container.querySelector<HTMLButtonElement>(".stanza-editor-fold-toggle");
	assert.ok(foldToggle);
	foldToggle.dispatchEvent(new dom.window.MouseEvent("pointerdown", { bubbles: true, cancelable: true }));
	assert.deepEqual([...container.querySelectorAll<HTMLElement>(".stanza-editor-line")].map(line => line.dataset.logicalLineIndex), ["0", "2"]);

	editorPart.dispose();
	dom.window.close();
});

test("Stanza editor disposal cancels an in-flight folding provider before late results project", async () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	const container = dom.window.document.querySelector<HTMLElement>("main")!;
	const model = new TextModel("root\n  child\nafter");
	const reference = modelReference(URI.file("C:\\project\\async-fold.txt"), model);
	using languageFeatures = new LanguageFeaturesService();
	let resolveRanges: ((ranges: readonly { readonly startLineIndex: number; readonly endLineIndex: number }[]) => void) | undefined;
	let providerSignal: AbortSignal | undefined;
	using registration = languageFeatures.registerFoldingRangeProvider({
		languageIds: ["plaintext"],
		provideFoldingRanges: (_request, signal) => {
			providerSignal = signal;
			return new Promise(resolve => { resolveRanges = resolve; });
		},
	});
	const errors: unknown[] = [];
	const editorPart = new EditorBrowser({ container, input: { resource: reference.resource, label: "async-fold.txt" }, languageId: "plaintext", modelReference: reference, languageFeaturesService: languageFeatures, onLanguageError: error => errors.push(error) });

	assert.equal(providerSignal?.aborted, false);
	editorPart.dispose();
	assert.equal(providerSignal?.aborted, true);
	resolveRanges?.([{ startLineIndex: 0, endLineIndex: 1 }]);
	await Promise.resolve();
	await Promise.resolve();

	assert.deepEqual(errors, []);
	assert.equal(container.children.length, 0);
	dom.window.close();
});

test("Stanza editor browser honors a read-only input without disabling selection infrastructure", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	const container = dom.window.document.querySelector<HTMLElement>("main")!;
	const model = new TextModel("alpha");
	const reference = modelReference(URI.file("C:\\project\\preview.txt"), model);
	const editorPart = new EditorBrowser({
		container,
		input: { resource: reference.resource, label: "preview.txt", readOnly: true },
		languageId: "plaintext",
		modelReference: reference,
	});

	const input = editorPart.textInput.element;
	assert.equal(input.readOnly, true);
	assert.equal(input.getAttribute("aria-readonly"), "true");
	const edit = new dom.window.InputEvent("beforeinput", {
		bubbles: true,
		cancelable: true,
		data: "x",
		inputType: "insertText",
	});
	input.dispatchEvent(edit);
	assert.equal(edit.defaultPrevented, true);
	assert.equal(editorPart.getValue(), "alpha");
	editorPart.selections.setSelections(editorPart.selections.selections);

	editorPart.dispose();
	dom.window.close();
});

test("Stanza editor browser mounts text drop as an optional full-editor contribution", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	const container = dom.window.document.querySelector<HTMLElement>("main")!;
	const model = new TextModel("alpha");
	const reference = modelReference(URI.file("C:\\project\\drop.txt"), model);
	const editorPart = new EditorBrowser({
		container,
		input: { resource: reference.resource, label: "drop.txt" },
		languageId: "plaintext",
		modelReference: reference,
	});
	editorPart.layout({ width: 120, height: 20 });
	editorPart.viewport.element.getBoundingClientRect = () => rectangle(120, 20);
	const drop = textDropEvent(dom.window, "dropped", 100, 5);

	editorPart.viewport.element.dispatchEvent(drop);

	assert.equal(drop.defaultPrevented, true);
	assert.equal(editorPart.getValue(), "alphadropped");
	editorPart.dispose();
	dom.window.close();
});

test("Stanza editor browser applies selected before-save contributions through explicit save", async () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	const container = dom.window.document.querySelector<HTMLElement>("main")!;
	const model = new TextModel("alpha");
	const reference = modelReference(URI.file("C:\\project\\save.txt"), model);
	using languageFeatures = new LanguageFeaturesService();
	using formatting = languageFeatures.registerFormattingProvider({
		languageIds: ["plaintext"],
		provideDocumentFormattingEdits: () => [{ range: TextRange.from(TextPosition.at(0, 0), TextPosition.at(0, 5)), text: "formatted" }],
	});
	let savedText = "";
	const editorPart = new EditorBrowser({
		container,
		input: { resource: reference.resource, label: "save.txt" },
		languageId: "plaintext",
		languageFeaturesService: languageFeatures,
		modelReference: reference,
		formatOnSave: true,
		insertFinalNewLine: true,
		onSave: async () => { savedText = model.getText(); },
	});
	await editorPart.save();
	assert.equal(savedText, "formatted\n");

	editorPart.dispose();
	dom.window.close();
});

test("Stanza editor browser omits disabled presentation and language-assistance contributions", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	const container = dom.window.document.querySelector<HTMLElement>("main")!;
	const model = new TextModel("function example() {\n  return 1;\n}");
	const reference = modelReference(URI.file("C:\\project\\minimal.ts"), model);
	const editorPart = new EditorBrowser({
		container,
		input: { resource: reference.resource, label: "minimal.ts" },
		languageId: "typescript",
		modelReference: reference,
		showLineNumbers: false,
		showIndentationGuides: false,
		bracketPairColorization: false,
		stickyScroll: false,
		suggestions: false,
		inlineCompletions: false,
		parameterHints: false,
		inlayHints: false,
		codeLens: false,
	});
	editorPart.layout({ width: 320, height: 80 });

	assert.equal(editorPart.viewport.element.classList.contains("hide-line-numbers"), true);
	assert.equal(container.querySelectorAll(".stanza-editor-indent-guide").length, 0);
	assert.equal(container.querySelectorAll(".stanza-editor-bracket-level-1").length, 0);
	assert.equal(container.querySelectorAll(".stanza-editor-sticky-scroll").length, 0);
	assert.equal(container.querySelectorAll(".stanza-editor-completion").length, 0);
	assert.equal(container.querySelectorAll(".stanza-editor-inline-completion").length, 0);
	assert.equal(container.querySelectorAll(".stanza-editor-parameter-hints").length, 0);

	editorPart.dispose();
	dom.window.close();
});

test("Code editor keeps large files editable while disabling full-document background features", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	const container = dom.window.document.querySelector<HTMLElement>("main")!;
	const model = new TextModel("let value = 1;\n".repeat(300_001));
	const reference = modelReference(URI.file("C:\\project\\large.ts"), model);
	const editorPart = new EditorBrowser({ container, input: { resource: reference.resource, label: "large.ts" }, languageId: "typescript", modelReference: reference });
	try {
		editorPart.layout({ width: 500, height: 40 });
		assert.equal(model.largeFile.tooLargeForTokenization, true, "large-file policy");
		assert.equal(container.querySelectorAll(".stanza-editor-token").length, 0, "background tokens");
		assert.equal(container.querySelectorAll(".stanza-editor-fold-toggle:not([hidden])").length, 0, "folding scan");
		editorPart.textInput.element.dispatchEvent(new dom.window.InputEvent("beforeinput", { bubbles: true, cancelable: true, data: "x", inputType: "insertText" }));
		assert.equal(editorPart.getValue().startsWith("xlet value = 1;\n"), true, "basic editing");
	} finally {
		editorPart.dispose();
		dom.window.close();
	}
});

test("constructor-backed editor contributions receive editor context and window services", async () => {
	const [{ EditorContributionInstantiation, registerEditorContribution }, { createServiceIdentifier, InstantiationService, ServiceCollection, SyncDescriptor }] = await Promise.all([
		import("../../browser/editorContribution.js"),
		import("../../../platform/instantiation/common/instantiation.js"),
	]);
	const IService = createServiceIdentifier<{ readonly value: string }>("editorRuntimeContributionTestService");
	let receivedResource = "";
	let receivedService = "";
	let disposed = false;
	class RuntimeContribution {
		constructor(context: { readonly options: { readonly input: { readonly resource: URI } } }, service: { readonly value: string }) {
			receivedResource = context.options.input.resource.toString();
			receivedService = service.value;
		}
		dispose(): void { disposed = true; }
		[Symbol.dispose](): void { this.dispose(); }
	}
	registerEditorContribution({
		id: "editor.contrib.runtimeInjection.test",
		runtime: {
			descriptor: new SyncDescriptor(RuntimeContribution, { serviceDependencies: [IService] }),
			instantiation: EditorContributionInstantiation.Eager,
		},
	});
	const services = new ServiceCollection();
	services.set(IService, { value: "window-service" });
	const instantiationService = new InstantiationService(services);
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	const model = new TextModel("runtime");
	const reference = modelReference(URI.file("C:\\project\\runtime.txt"), model);
	const editorPart = new EditorBrowser({
		container: dom.window.document.querySelector<HTMLElement>("main")!,
		input: { resource: reference.resource },
		languageId: "plaintext",
		modelReference: reference,
		instantiationService,
	});

	assert.equal(receivedResource, reference.resource.toString());
	assert.equal(receivedService, "window-service");
	assert.equal(disposed, false);
	editorPart.dispose();
	assert.equal(disposed, true);
	dom.window.close();
});

function modelReference(resource: URI, model: TextModel): TextModelReference {
	let disposed = false;
	const dispose = (): void => {
		if (disposed) return;
		disposed = true;
		model.dispose();
	};
	return {
		resource,
		model,
		get isDirty(): boolean {
			return false;
		},
		onDidChangeDirty: () => ({
			dispose() {},
			[Symbol.dispose]() {},
		}),
		get hasExternalChange(): boolean {
			return false;
		},
		onDidChangeExternalChange: () => ({
			dispose() {},
			[Symbol.dispose]() {},
		}),
		async save(): Promise<void> {},
		async revert(): Promise<void> {},
		dispose,
		[Symbol.dispose]: dispose,
	};
}

function nextTask(): Promise<void> {
	return new Promise(resolve => setTimeout(resolve, 0));
}

function textDropEvent(targetWindow: typeof browserEnvironment.window, text: string, clientX: number, clientY: number): DragEvent {
	const event = new targetWindow.Event("drop", { bubbles: true, cancelable: true });
	Object.defineProperties(event, {
		clientX: { value: clientX },
		clientY: { value: clientY },
		dataTransfer: {
			value: {
				types: ["text/plain"],
				getData(type: string): string {
					return type === "text/plain" ? text : "";
				},
			},
		},
	});
	return event as unknown as DragEvent;
}

function rectangle(width: number, height: number): DOMRect {
	return {
		x: 0,
		y: 0,
		width,
		height,
		top: 0,
		right: width,
		bottom: height,
		left: 0,
		toJSON: () => ({}),
	};
}

async function waitFor(predicate: () => boolean): Promise<void> {
	for (let attempt = 0; attempt < 20; attempt += 1) {
		if (predicate()) return;
		await nextTask();
	}
	assert.fail("Timed out waiting for Stanza editor projection");
}
