import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { DisposableTracker, installDisposableTracker } from "../../../base/common/lifecycle.js";
import { URI } from "../../../base/common/uri.js";
import { registerBuiltinLanguageConfigurations } from "../../common/languages/languageBuiltinConfigurations.js";
import { LanguageFeaturesService } from "../../common/services/languageFeaturesService.js";
import { LanguageConfigurationService } from '../../common/services/languageConfigurationService.js';
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
	const resource = URI.file("C:\\project\\settings.json");
	const errors: unknown[] = [];
	using languageConfigurationService = new LanguageConfigurationService();
	using languageFeaturesService = new LanguageFeaturesService(languageConfigurationService);
	using languageConfigurations = registerBuiltinLanguageConfigurations(languageConfigurationService.configurations);
	const editorPart = new EditorBrowser({
		container,
		input: {
			resource,
			label: "settings.json",
		},
		languageId: "json",
		model,
		languageFeaturesService,
		languageConfigurationService,
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

	editorPart.view.element.dispatchEvent(new dom.window.InputEvent("beforeinput", {
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
	assert.equal(model.getText().startsWith("x{"), true);
	model.dispose();
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
		const resource = URI.file("C:\\project\\main.ts");
		const editorPart = new EditorBrowser({ container, input: { resource, label: "main.ts" }, languageId: "typescript", model });

		editorPart.dispose();
		model.dispose();
		dom.window.close();
	}

	assert.deepEqual(tracker.leaks().filter(leak => leak.label === "LanguageEditingAdapter"), []);
});

test("Stanza editor browser derives indentation folds and projects their gutter controls", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	const container = dom.window.document.querySelector<HTMLElement>("main")!;
	const model = new TextModel("root\n  child\nafter");
	const resource = URI.file("C:\\project\\fold.txt");
	const editorPart = new EditorBrowser({
		container,
		input: {
			resource,
			label: "fold.txt",
		},
		languageId: "plaintext",
		model,
	});
	editorPart.layout({ width: 500, height: 120 });

	const foldToggle = container.querySelector<HTMLButtonElement>(".stanza-editor-fold-toggle");
	assert.ok(foldToggle);
	foldToggle.dispatchEvent(new dom.window.MouseEvent("pointerdown", { bubbles: true, cancelable: true }));
	assert.deepEqual([...container.querySelectorAll<HTMLElement>(".stanza-editor-line")].map(line => line.dataset.logicalLineIndex), ["0", "2"]);

	editorPart.dispose();
	model.dispose();
	dom.window.close();
});

test("Stanza editor browser marks named regions and comment MARK headers only", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	const container = dom.window.document.querySelector<HTMLElement>("main")!;
	const model = new TextModel("// #region Runtime\nconst folded = {\n};\n// #endregion\nconst marker = 1; // MARK: API\nconst fake = 'MARK: not a header';");
	using languageConfigurationService = new LanguageConfigurationService();
	using languageFeaturesService = new LanguageFeaturesService(languageConfigurationService);
	using languageConfigurations = registerBuiltinLanguageConfigurations(languageConfigurationService.configurations);
	const editorPart = new EditorBrowser({
		container,
		input: { resource: URI.file("C:\\project\\sections.ts"), label: "sections.ts" },
		languageId: "typescript",
		model,
		languageFeaturesService,
		languageConfigurationService,
	});
	editorPart.layout({ width: 500, height: 180 });

	assert.deepEqual([...container.querySelectorAll<HTMLElement>(".stanza-editor-line.section-header")].map(line => Number(line.dataset.logicalLineIndex)), [0, 4]);

	editorPart.dispose();
	model.dispose();
	dom.window.close();
});

test("Stanza editor disposal cancels an in-flight folding provider before late results project", async () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	const container = dom.window.document.querySelector<HTMLElement>("main")!;
	const model = new TextModel("root\n  child\nafter");
	const resource = URI.file("C:\\project\\async-fold.txt");
	using languageConfigurationService = new LanguageConfigurationService();
	using languageFeatures = new LanguageFeaturesService(languageConfigurationService);
	let resolveRanges: ((ranges: readonly { readonly startLineIndex: number; readonly endLineIndex: number }[]) => void) | undefined;
	let providerSignal: AbortSignal | undefined;
	using registration = languageFeatures.foldingRangeProvider.register({
		languageIds: ["plaintext"],
		provideFoldingRanges: (_request, signal) => {
			providerSignal = signal;
			return new Promise(resolve => { resolveRanges = resolve; });
		},
	});
	const errors: unknown[] = [];
	const editorPart = new EditorBrowser({ container, input: { resource, label: "async-fold.txt" }, languageId: "plaintext", model, languageFeaturesService: languageFeatures, languageConfigurationService, onLanguageError: error => errors.push(error) });

	assert.equal(providerSignal?.aborted, false);
	editorPart.dispose();
	assert.equal(providerSignal?.aborted, true);
	resolveRanges?.([{ startLineIndex: 0, endLineIndex: 1 }]);
	await Promise.resolve();
	await Promise.resolve();

	assert.deepEqual(errors, []);
	assert.equal(container.children.length, 0);
	model.dispose();
	dom.window.close();
});

test("Stanza editor browser honors a read-only input without disabling selection infrastructure", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	const container = dom.window.document.querySelector<HTMLElement>("main")!;
	const model = new TextModel("alpha");
	const resource = URI.file("C:\\project\\preview.txt");
	const editorPart = new EditorBrowser({
		container,
		input: { resource, label: "preview.txt", readOnly: true },
		languageId: "plaintext",
		model,
	});

	const input = editorPart.view.textArea!;
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
	model.dispose();
	dom.window.close();
});

test("Stanza editor browser mounts text drop as an optional full-editor contribution", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	const container = dom.window.document.querySelector<HTMLElement>("main")!;
	const model = new TextModel("alpha");
	const resource = URI.file("C:\\project\\drop.txt");
	const editorPart = new EditorBrowser({
		container,
		input: { resource, label: "drop.txt" },
		languageId: "plaintext",
		model,
		glyphMargin: false,
	});
	editorPart.layout({ width: 120, height: 20 });
	editorPart.viewport.element.getBoundingClientRect = () => rectangle(120, 20);
	const dropPosition = editorPart.viewport.getPositionContentCoordinates(TextPosition.at(0, 5));
	const drop = textDropEvent(dom.window, "dropped", dropPosition.left, dropPosition.top + dropPosition.height / 2);

	editorPart.viewport.element.dispatchEvent(drop);

	assert.equal(drop.defaultPrevented, true);
	assert.equal(editorPart.getValue(), "alphadropped");
	editorPart.dispose();
	model.dispose();
	dom.window.close();
});

test("Stanza editor browser prepares selected before-save contributions for host persistence", async () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	const container = dom.window.document.querySelector<HTMLElement>("main")!;
	const model = new TextModel("alpha");
	const resource = URI.file("C:\\project\\save.txt");
	using languageConfigurationService = new LanguageConfigurationService();
	using languageFeatures = new LanguageFeaturesService(languageConfigurationService);
	using formatting = languageFeatures.formattingProvider.register({
		languageIds: ["plaintext"],
		provideDocumentFormattingEdits: () => [{ range: TextRange.from(TextPosition.at(0, 0), TextPosition.at(0, 5)), text: "formatted" }],
	});
	const editorPart = new EditorBrowser({
		container,
		input: { resource, label: "save.txt" },
		languageId: "plaintext",
		languageFeaturesService: languageFeatures,
		languageConfigurationService,
		model,
		formatOnSave: true,
		insertFinalNewLine: true,
	});
	await editorPart.prepareSave();
	assert.equal(model.getText(), "formatted\n");

	editorPart.dispose();
	model.dispose();
	dom.window.close();
});

test("Stanza editor browser omits disabled presentation and language-assistance contributions", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	const container = dom.window.document.querySelector<HTMLElement>("main")!;
	const model = new TextModel("function example() {\n  return 1;\n}");
	const resource = URI.file("C:\\project\\minimal.ts");
	const editorPart = new EditorBrowser({
		container,
		input: { resource, label: "minimal.ts" },
		languageId: "typescript",
		model,
		lineNumbers: 'off',
		showSymbolIcons: false,
		guides: { indentation: false },
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
	assert.equal(editorPart.viewport.element.style.getPropertyValue("--stanza-editor-line-decorations-width"), "20px");
	assert.equal(container.querySelectorAll(".stanza-editor-indent-guide").length, 0);
	assert.equal(container.querySelectorAll(".stanza-editor-bracket-level-1").length, 0);
	assert.equal(container.querySelectorAll(".stanza-editor-sticky-scroll").length, 0);
	assert.equal(container.querySelectorAll(".stanza-editor-completion").length, 0);
	assert.equal(container.querySelectorAll(".stanza-editor-inline-completion").length, 0);
	assert.equal(container.querySelectorAll(".stanza-editor-parameter-hints").length, 0);

	editorPart.dispose();
	model.dispose();
	dom.window.close();
});

test("Code editor keeps large files editable while disabling full-document background features", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	const container = dom.window.document.querySelector<HTMLElement>("main")!;
	const model = new TextModel("let value = 1;\n".repeat(300_001));
	const resource = URI.file("C:\\project\\large.ts");
	const editorPart = new EditorBrowser({ container, input: { resource, label: "large.ts" }, languageId: "typescript", model });
	try {
		editorPart.layout({ width: 500, height: 40 });
		assert.equal(model.largeFile.tooLargeForTokenization, true, "large-file policy");
		assert.equal(container.querySelectorAll(".stanza-editor-token").length, 0, "background tokens");
		assert.equal(container.querySelectorAll(".stanza-editor-fold-toggle:not([hidden])").length, 0, "folding scan");
		editorPart.view.element.dispatchEvent(new dom.window.InputEvent("beforeinput", { bubbles: true, cancelable: true, data: "x", inputType: "insertText" }));
		assert.equal(editorPart.getValue().startsWith("xlet value = 1;\n"), true, "basic editing");
	} finally {
		editorPart.dispose();
		model.dispose();
		dom.window.close();
	}
});

test("constructor-backed editor contributions receive editor context and window services", async () => {
const [{ EditorContributionInstantiation, registerEditorContribution }, { createServiceIdentifier, ServiceContainer, SyncDescriptor }] = await Promise.all([
		import("../../browser/editorExtensions.js"),
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
	const services = new ServiceContainer();
	services.registerInstance(IService, { value: "window-service" });
	const instantiationService = services;
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	const model = new TextModel("runtime");
	const resource = URI.file("C:\\project\\runtime.txt");
	const editorPart = new EditorBrowser({
		container: dom.window.document.querySelector<HTMLElement>("main")!,
		input: { resource },
		languageId: "plaintext",
		model,
		instantiationService,
	});

	assert.equal(receivedResource, resource.toString());
	assert.equal(receivedService, "window-service");
	assert.equal(disposed, false);
	editorPart.dispose();
	model.dispose();
	assert.equal(disposed, true);
	dom.window.close();
});

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
