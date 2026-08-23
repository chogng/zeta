import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { Emitter } from "../../../../../base/common/event.js";
import { toDisposable } from "../../../../../base/common/lifecycle.js";
import { URI } from "../../../../../base/common/uri.js";
import { TextPosition, TextRange } from "../../../../../editor/common/core/text.js";
import { type TextModel } from "../../../../../editor/common/model/textModel.js";
import { type ITextModelService, type TextModelReference } from "../../../../../editor/common/services/textModelService.js";
import { type LanguageWorkspaceEdit } from "../../../../../editor/common/languages/languageWorkspaceEdit.js";
import { FileKind, FileNotFoundError, type IFileService } from "../../../../../platform/files/common/files.js";
import { type IWorkingCopyService } from "../../../../services/workingCopy/common/workingCopyService.js";
import { type BulkEditPreviewModel } from "../../common/bulkEdit.js";

test("bulk edit preview follows ordered create and text operations without mutating files", async () => {
	const browser = new JSDOM("<!doctype html><body></body>");
	const installedGlobals = installDomGlobals(browser);
	const existing = URI.file("C:\\workspace\\existing.ts");
	const created = URI.file("C:\\workspace\\created.ts");
	const files = new PreviewFileService([[existing, "alpha"]]);
	const models = new PreviewTextModelService([[existing, "alpha"]], [existing]);
	const edit: LanguageWorkspaceEdit = {
		entries: [
			{ kind: "create", resource: created, existing: "error" },
			{ kind: "textDocument", resource: created, expectedText: "", edits: [{ range: TextRange.emptyAt(TextPosition.at(0, 0)), text: "hello" }] },
			{ kind: "textDocument", resource: existing, expectedText: "alpha", edits: [{ range: TextRange.from(TextPosition.at(0, 0), TextPosition.at(0, 5)), text: "omega" }] },
			{ kind: "textDocument", resource: existing, expectedText: "omega", edits: [{ range: TextRange.emptyAt(TextPosition.at(0, 5)), text: "!" }] },
		],
	};

	try {
		const { createBulkEditPreview } = await import("../../browser/preview/bulkEditPreview.js");
		const preview = await createBulkEditPreview(edit, { files, models, workingCopies: emptyWorkingCopies() }, new AbortController().signal);

		assert.equal(preview.canApply, true);
		assert.equal(preview.entries.every(entry => entry.error === undefined), true);
		assert.equal(preview.entries[1]?.before, "");
		assert.equal(preview.entries[1]?.after, "hello");
		assert.equal(preview.entries[2]?.before, "alpha");
		assert.equal(preview.entries[2]?.after, "omega");
		assert.equal(preview.entries[3]?.before, "omega");
		assert.equal(preview.entries[3]?.after, "omega!");
		assert.equal(files.read(existing), "alpha");
		assert.equal(files.has(created), false);
	} finally {
		for (const name of installedGlobals) Reflect.deleteProperty(globalThis, name);
		browser.window.close();
	}
});

class PreviewTextModelService implements ITextModelService {
	readonly references: TextModelReference[] = [];
	private readonly resources: ReadonlyMap<string, string>;
	private readonly persistentResources: ReadonlySet<string>;
	private readonly persistentModels = new Map<string, TextModel>();

	constructor(resources: readonly (readonly [URI, string])[], persistentResources: readonly URI[] = []) {
		this.resources = new Map(resources.map(([resource, text]) => [resource.toString(), text]));
		this.persistentResources = new Set(persistentResources.map(resource => resource.toString()));
	}

	async acquire(input: { readonly resource: URI; readonly initialText?: string }): Promise<TextModelReference> {
		const { TextModel } = await import("../../../../../editor/common/model/textModel.js");
		const key = input.resource.toString();
		const model = this.persistentModels.get(key) ?? new TextModel(input.initialText ?? this.resources.get(key) ?? "");
		const persistent = this.persistentResources.has(key);
		if (persistent) this.persistentModels.set(key, model);
		const emptyEvent = () => toDisposable(() => undefined);
		const reference: TextModelReference = {
			resource: input.resource,
			model,
			isDirty: false,
			onDidChangeDirty: emptyEvent,
			hasExternalChange: false,
			onDidChangeExternalChange: emptyEvent,
			save: async () => undefined,
			revert: async () => undefined,
			dispose: () => { if (!persistent) model.dispose(); },
			[Symbol.dispose]() { if (!persistent) model.dispose(); },
		};
		this.references.push(reference);
		return reference;
	}

	dispose(): void {
		for (const reference of this.references) reference.dispose();
		this.references.length = 0;
	}

	[Symbol.dispose](): void { this.dispose(); }
}

class PreviewFileService implements IFileService {
	readonly onDidChangeFiles = new Emitter<{ readonly resources: readonly URI[] | undefined }>().event;
	private readonly resources = new Map<string, string>();

	constructor(resources: readonly (readonly [URI, string])[]) {
		for (const [resource, text] of resources) this.resources.set(resource.toString(), text);
	}

	has(resource: URI): boolean { return this.resources.has(resource.toString()); }
	read(resource: URI): string { return this.resources.get(resource.toString()) ?? ""; }
	async stat(resource: URI) {
		if (!this.has(resource)) throw new FileNotFoundError(resource);
		return { resource, kind: FileKind.File, sizeBytes: this.read(resource).length, readonly: false, modifiedAtMillis: undefined };
	}
	async readDirectory(): Promise<readonly never[]> { return []; }
	async readFile(resource: URI) { if (!this.has(resource)) throw new FileNotFoundError(resource); return { resource, content: this.read(resource), revision: "1" }; }
	async readFileBytes(resource: URI) { const content = await this.readFile(resource); return { resource, bytes: new TextEncoder().encode(content.content), revision: content.revision }; }
	async writeFile(): Promise<never> { throw new Error("Preview must not write files"); }
	async createFile(): Promise<never> { throw new Error("Preview must not create files"); }
	async rename(): Promise<never> { throw new Error("Preview must not rename files"); }
	async delete(): Promise<never> { throw new Error("Preview must not delete files"); }
}

function emptyWorkingCopies(): IWorkingCopyService {
	const emptyEvent = () => toDisposable(() => undefined);
	return {
		onDidRegister: emptyEvent,
		onDidUnregister: emptyEvent,
		register: () => toDisposable(() => undefined),
		get: () => [],
		getAll: () => [],
		dispose() {},
		[Symbol.dispose]() {},
	};
}

function installDomGlobals(browser: JSDOM): readonly string[] {
	const globals = {
		window: browser.window,
		document: browser.window.document,
		Node: browser.window.Node,
		Element: browser.window.Element,
		HTMLElement: browser.window.HTMLElement,
		Event: browser.window.Event,
		MouseEvent: browser.window.MouseEvent,
		navigator: browser.window.navigator,
	};
	for (const [name, value] of Object.entries(globals)) Object.defineProperty(globalThis, name, { configurable: true, value });
	return Object.keys(globals);
}
