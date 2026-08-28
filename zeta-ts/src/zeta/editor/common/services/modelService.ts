import { Emitter } from "../../../base/common/event.js";
import { Disposable, DisposableMap } from "../../../base/common/lifecycle.js";
import { URI } from "../../../base/common/uri.js";
import { TextModel } from "../model/textModel.js";
import type { IModelService, ModelLanguageChangeEvent } from "./model.js";

/** Resource and language registry for caller-owned TextModels. */
export class ModelService extends Disposable implements IModelService {
	private readonly entries = this._register(new DisposableMap<string, ModelEntry>());
	private readonly entriesByModel = new WeakMap<TextModel, ModelEntry>();
	private readonly createEmitter = this._register(new Emitter<TextModel>());
	private readonly willDisposeEmitter = this._register(new Emitter<TextModel>());
	private readonly languageEmitter = this._register(new Emitter<ModelLanguageChangeEvent>());
	private modelIdentity = 1;

	readonly onDidCreateModel = this.createEmitter.event;
	readonly onWillDisposeModel = this.willDisposeEmitter.event;
	readonly onDidChangeModelLanguage = this.languageEmitter.event;

	createModel(value: string, languageId = "plaintext", resource = this.nextResource()): TextModel {
		if (typeof value !== "string") throw new TypeError("Model value must be a string");
		const normalizedLanguageId = requireLanguageId(languageId);
		const key = resource.toString();
		if (this.entries.has(key)) throw new Error(`A model already exists for '${key}'`);
		const model = new TextModel(value);
		try {
			const entry = new ModelEntry(model, resource, normalizedLanguageId, () => this.removeEntry(key, model));
			this.entries.set(key, entry);
			this.entriesByModel.set(model, entry);
			this.createEmitter.fire(model);
			return model;
		} catch (error) {
			model.dispose();
			throw error;
		}
	}

	getModel(resource: URI): TextModel | undefined {
		return this.entryForResource(resource)?.model;
	}

	getModels(): readonly TextModel[] {
		return Object.freeze([...this.entries].map(([, entry]) => entry.model));
	}

	getModelResource(model: TextModel): URI {
		return this.requireEntry(model).resource;
	}

	getModelLanguage(model: TextModel): string {
		return this.requireEntry(model).languageId;
	}

	setModelLanguage(model: TextModel, languageId: string): void {
		const entry = this.requireEntry(model);
		const nextLanguageId = requireLanguageId(languageId);
		const oldLanguageId = entry.languageId;
		if (oldLanguageId === nextLanguageId) return;
		entry.languageId = nextLanguageId;
		this.languageEmitter.fire(Object.freeze({ model, oldLanguageId, newLanguageId: nextLanguageId }));
	}

	private nextResource(): URI {
		return URI.parse(`inmemory://stanza/model/${this.modelIdentity++}`);
	}

	private entryForResource(resource: URI): ModelEntry | undefined {
		const key = resource.toString();
		for (const [candidate, entry] of this.entries) if (candidate === key) return entry;
		return undefined;
	}

	private requireEntry(model: TextModel): ModelEntry {
		const entry = this.entriesByModel.get(model);
		if (!entry) throw new ReferenceError("TextModel is not registered with the model service");
		return entry;
	}

	private removeEntry(key: string, model: TextModel): void {
		const entry = this.entriesByModel.get(model);
		if (!entry || entry.resource.toString() !== key) return;
		this.willDisposeEmitter.fire(model);
		this.entriesByModel.delete(model);
		this.entries.deleteAndDispose(key);
	}
}

class ModelEntry extends Disposable {
	constructor(
		readonly model: TextModel,
		readonly resource: URI,
		public languageId: string,
		onWillDispose: () => void,
	) {
		super();
		this._register(model.onWillDispose(onWillDispose));
	}
}

function requireLanguageId(languageId: string): string {
	if (typeof languageId !== "string" || languageId.trim().length === 0) {
		throw new TypeError("Model language id must be a non-empty string");
	}
	return languageId;
}
