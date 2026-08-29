import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { URI } from "../../../base/common/uri.js";
import { lightColorTheme } from "../../../platform/theme/common/colorTheme.js";
import { LanguageFeaturesService } from "../../common/services/languageFeaturesService.js";
import { LanguageConfigurationService } from '../../common/services/languageConfigurationService.js';
import { HoverService } from '../../contrib/hover/common/hover.js';
import { StandaloneServiceCollection, StandaloneServices } from "../../standalone/browser/standaloneServices.js";

const browserEnvironment = new JSDOM("<!doctype html><body></body>");
const forcedColors = new browserEnvironment.window.EventTarget();
Object.defineProperties(forcedColors, {
	matches: { configurable: true, value: false, writable: true },
	media: { configurable: true, value: "(forced-colors: active)" },
});
Object.defineProperty(browserEnvironment.window, "matchMedia", {
	configurable: true,
	value: (query: string) => {
		assert.equal(query, "(forced-colors: active)");
		return forcedColors;
	},
});
let createdWorkerCount = 0;
let terminatedWorkerCount = 0;
class TestWorker extends browserEnvironment.window.EventTarget {
	constructor() {
		super();
		createdWorkerCount += 1;
	}
	postMessage(): void {}
	terminate(): void { terminatedWorkerCount += 1; }
}
for (const [name, value] of Object.entries({
	window: browserEnvironment.window,
	document: browserEnvironment.window.document,
	Node: browserEnvironment.window.Node,
	Element: browserEnvironment.window.Element,
	HTMLElement: browserEnvironment.window.HTMLElement,
	Event: browserEnvironment.window.Event,
	InputEvent: browserEnvironment.window.InputEvent,
	Worker: TestWorker,
})) Object.defineProperty(globalThis, name, { configurable: true, value });

const stanza = await import("../../editor.main.js");

test.after(() => browserEnvironment.window.close());

test("standalone service collection honors explicit first-scope overrides", () => {
	const languageConfigurations = new LanguageConfigurationService();
	const languages = new LanguageFeaturesService(languageConfigurations);
	const services = new StandaloneServiceCollection({ languageConfigurationService: languageConfigurations, languageFeaturesService: languages });
	assert.equal(services.languageFeaturesService, languages);
	assert.equal(services.themeService.getColorTheme(), lightColorTheme);
	services.dispose();
	assert.equal(languages.isDisposed, false);
	languages.dispose();
	languageConfigurations.dispose();
});

test("standalone theme APIs register, select, and project a named theme", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	stanza.editor.defineTheme("standalone-test", {
		label: "Standalone Test",
		colorScheme: stanza.ColorScheme.Dark,
		colors: { "editor.background": "#101010" },
	});
	const container = dom.window.document.querySelector<HTMLElement>("main")!;
	const editor = stanza.editor.create(container, { value: "theme", theme: "standalone-test" });
	assert.equal(container.getAttribute("data-color-theme"), "standalone-test");
	assert.equal(container.style.getPropertyValue("--zeta-editor-background"), "#101010");

	stanza.editor.setTheme("zeta-light");
	assert.equal(container.getAttribute("data-color-theme"), "zeta-light");
	editor.dispose();
	dom.window.close();
});

test("standalone public API keeps compiled theme snapshots internal", () => {
	for (const exportName of ["lightColorTheme", "darkColorTheme", "highContrastLightColorTheme", "highContrastDarkColorTheme"]) {
		assert.equal(exportName in stanza, false);
	}
	assert.equal(stanza.editor.ContentWidgetPositionPreference.EXACT, stanza.ContentWidgetPositionPreference.EXACT);
	assert.equal(stanza.editor.OverlayWidgetPositionPreference.TOP_CENTER, stanza.OverlayWidgetPositionPreference.TOP_CENTER);
	assert.equal(stanza.editor.PositionAffinity.LeftOfInjectedText, stanza.PositionAffinity.LeftOfInjectedText);
});

test("standalone languages API exposes provider value types", () => {
	assert.equal(stanza.languages.LanguageCompletionItemKind, stanza.LanguageCompletionItemKind);
	assert.equal(stanza.languages.LanguageCompletionInsertTextFormat, stanza.LanguageCompletionInsertTextFormat);
	assert.equal(stanza.languages.LanguageCompletionTriggerKind, stanza.LanguageCompletionTriggerKind);
	assert.equal(stanza.languages.LanguageDiagnosticSeverity, stanza.LanguageDiagnosticSeverity);
	assert.equal(stanza.languages.DocumentHighlightKind, stanza.DocumentHighlightKind);
	assert.equal(stanza.languages.RGBA8, stanza.RGBA8);
	assert.deepEqual(new stanza.languages.RGBA8(300, -1, 64, 255), new stanza.RGBA8(255, 0, 64, 255));
});

test('standalone languages API replaces one language generation without stale registrations', () => {
	const changes: string[] = [];
	using listener = stanza.languages.onDidChangeLanguages(event => changes.push(event.languageId));
	using descriptions = stanza.languages.registerLanguages([{
		description: { id: 'stanza-generation-a', extensions: ['.generation-a'] },
	}]);

	assert.equal(stanza.languages.resolveLanguageId({ resource: URI.parse('file:///sample.generation-a') }), 'stanza-generation-a');
	descriptions.replace([{
		description: { id: 'stanza-generation-b', extensions: ['.generation-b'] },
	}]);
	assert.equal(stanza.languages.resolveLanguageId({ resource: URI.parse('file:///sample.generation-a') }), undefined);
	assert.equal(stanza.languages.resolveLanguageId({ resource: URI.parse('file:///sample.generation-b') }), 'stanza-generation-b');
	assert.deepEqual(changes, ['stanza-generation-a', 'stanza-generation-a', 'stanza-generation-b']);
});

test('standalone languages API replaces provider batches atomically', () => {
	const first = {
		providerId: 'stanza.batch.first',
		languageIds: ['stanza-batch'],
		provideHover: () => ({ contents: ['first'] }),
	};
	const second = {
		providerId: 'stanza.batch.second',
		languageIds: ['stanza-batch'],
		provideHover: () => ({ contents: ['second'] }),
	};
	using batch = stanza.languages.registerProviderBatch({ hovers: [first] });
	const providers = StandaloneServices.get().languageFeaturesService.hoverProvider;

	assert.deepEqual(providers.getProviders('stanza-batch'), [first]);
	batch.replace({ hovers: [second] });
	assert.deepEqual(providers.getProviders('stanza-batch'), [second]);
});

test("standalone languages API feeds the shared editor registries", async () => {
	using language = stanza.languages.register({ id: 'stanza-public-test', extensions: ['.stanza-public'] });
	using configuration = stanza.languages.setLanguageConfiguration('stanza-public-test', { comments: { lineComment: '//' } });
	using provider = stanza.languages.registerHoverProvider({
		languageIds: ['stanza-public-test'],
		provideHover: () => ({ contents: ['Public hover'] }),
	});
	const services = StandaloneServices.get();
	assert.equal(services.languageService.resolveLanguageId({ resource: URI.parse('file:///sample.stanza-public') }), 'stanza-public-test');
	assert.equal(services.languageConfigurationService.getLanguageConfiguration('stanza-public-test').comments.lineComment, '//');
	using model = stanza.editor.createModel('answer', 'stanza-public-test', URI.parse('inmemory://stanza/public-api.stanza-public'));
	using hover = new HoverService(model, services.languageFeaturesService.hoverProvider);
	assert.deepEqual(await hover.provideHover('stanza-public-test', stanza.TextPosition.at(0, 1)), { contents: ['Public hover'] });
});

test("standalone completion providers execute in a live editor", async () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	let requests = 0;
	using language = stanza.languages.register({ id: "stanza-completion-test" });
	using provider = stanza.languages.registerCompletionProvider({
		id: "standalone.test",
		languageIds: ["stanza-completion-test"],
		provideCompletions: request => {
			requests += 1;
			assert.equal(request.context.kind, stanza.languages.LanguageCompletionTriggerKind.Invoke);
			return {
				items: [{
					id: "standalone-result",
					label: "standaloneResult",
					kind: stanza.languages.LanguageCompletionItemKind.Text,
					range: stanza.TextRange.emptyAt(request.position),
					insertText: "standaloneResult",
					insertTextFormat: stanza.languages.LanguageCompletionInsertTextFormat.PlainText,
				}],
				isIncomplete: false,
			};
		},
	});
	const container = dom.window.document.querySelector<HTMLElement>("main")!;
	const editor = stanza.editor.create(container, { languageId: "stanza-completion-test" });

	editor.view.element.dispatchEvent(new dom.window.KeyboardEvent("keydown", {
		bubbles: true,
		cancelable: true,
		ctrlKey: true,
		key: " ",
	}));
	await new Promise<void>(resolve => setImmediate(resolve));
	await new Promise<void>(resolve => setImmediate(resolve));

	assert.equal(requests, 1);
	assert.equal(container.querySelector(".stanza-editor-completion-option")?.textContent, "TextstandaloneResult");
	editor.dispose();
	dom.window.close();
});

test("standalone API registers URI and language identity with model lifecycle events", () => {
	const resource = URI.parse("inmemory://stanza/registry.ts");
	const created: string[] = [];
	const disposed: string[] = [];
	const languages: string[] = [];
	using createListener = stanza.editor.onDidCreateModel(model => created.push(stanza.editor.getModelResource(model).toString()));
	using disposeListener = stanza.editor.onWillDisposeModel(model => disposed.push(stanza.editor.getModelResource(model).toString()));
	using languageListener = stanza.editor.onDidChangeModelLanguage(event => languages.push(`${event.oldLanguageId}->${event.newLanguageId}`));

	const model = stanza.editor.createModel("const value = 1;", "typescript", resource);
	assert.equal(stanza.editor.getModel(resource), model);
	assert.equal(stanza.editor.getModelLanguage(model), "typescript");
	assert.deepEqual(created, [resource.toString()]);
	assert.throws(() => stanza.editor.createModel("duplicate", "typescript", resource), /already exists/);

	stanza.editor.setModelLanguage(model, "javascript");
	assert.deepEqual(languages, ["typescript->javascript"]);
	model.dispose();
	assert.equal(stanza.editor.getModel(resource), undefined);
	assert.deepEqual(disposed, [resource.toString()]);
});

test("standalone editors share caller-owned models and dispose independently", () => {
	const dom = new JSDOM("<!doctype html><body><main></main><aside></aside></body>");
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	const model = stanza.editor.createModel("shared", "plaintext", URI.parse("inmemory://stanza/shared.txt"));
	const first = stanza.editor.create(dom.window.document.querySelector<HTMLElement>("main")!, { model });
	const second = stanza.editor.create(dom.window.document.querySelector<HTMLElement>("aside")!, { model });

	assert.equal(stanza.editor.getEditors().includes(first), true);
	assert.equal(stanza.editor.getEditors().includes(second), true);
	first.setValue("shared model");
	assert.equal(second.getValue(), "shared model");
	first.dispose();
	assert.equal(model.getText(), "shared model");
	assert.equal(stanza.editor.getEditors().includes(first), false);

	second.dispose();
	assert.equal(model.getText(), "shared model");
	model.dispose();
	dom.window.close();
});

test("standalone editor owns only the implicit model it creates", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	const editor = stanza.editor.create(dom.window.document.querySelector<HTMLElement>("main")!, {
		value: "owned",
		languageId: "plaintext",
		resource: URI.parse("inmemory://stanza/owned.txt"),
	});
	const model = editor.getModel();

	editor.dispose();
	assert.equal(model.isDisposed, true);
	assert.equal(createdWorkerCount, terminatedWorkerCount);
	assert.equal(stanza.editor.getModel(URI.parse("inmemory://stanza/owned.txt")), undefined);
	dom.window.close();
});

test("standalone editor rejects unregistered models and conflicting model options", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const model = new stanza.TextModel("unregistered");
	assert.throws(() => stanza.editor.create(dom.window.document.querySelector<HTMLElement>("main")!, { model }), /not registered/);
	model.dispose();

	const registered = stanza.editor.createModel("registered", "plaintext", URI.parse("inmemory://stanza/conflict.txt"));
	assert.throws(() => stanza.editor.create(dom.window.document.querySelector<HTMLElement>("main")!, { model: registered, value: "conflict" }), /cannot be combined/);
	registered.dispose();
	const lateConfigurations = new LanguageConfigurationService();
	const lateOverride = new LanguageFeaturesService(lateConfigurations);
	assert.throws(() => stanza.editor.create(dom.window.document.querySelector<HTMLElement>("main")!, {}, { languageFeaturesService: lateOverride }), /already initialized/);
	lateOverride.dispose();
	lateConfigurations.dispose();
	dom.window.close();
});
