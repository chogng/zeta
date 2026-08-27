import { Emitter, type Event } from "../../../base/common/event.js";
import { Disposable, DisposableMap } from "../../../base/common/lifecycle.js";
import { URI } from "../../../base/common/uri.js";
import { TextModel } from "../../common/model/textModel.js";
import { createServiceIdentifier } from "../../../platform/instantiation/common/instantiation.js";

export interface StandaloneModelLanguageChangeEvent {
	readonly model: TextModel;
	readonly oldLanguageId: string;
	readonly newLanguageId: string;
}

export interface IStandaloneModelService {
	readonly onDidCreateModel: Event<TextModel>;
	readonly onWillDisposeModel: Event<TextModel>;
	readonly onDidChangeModelLanguage: Event<StandaloneModelLanguageChangeEvent>;
	createModel(value: string, languageId?: string, resource?: URI): TextModel;
	getModel(resource: URI): TextModel | undefined;
	getModels(): readonly TextModel[];
	getModelResource(model: TextModel): URI;
	getModelLanguage(model: TextModel): string;
	setModelLanguage(model: TextModel, languageId: string): void;
}

export const IStandaloneModelService = createServiceIdentifier<IStandaloneModelService>("standaloneModelService");

/** URI and language registry for caller-owned standalone TextModels. */
export class StandaloneModelService extends Disposable implements IStandaloneModelService {
	private readonly entries = this._register(new DisposableMap<string, StandaloneModelEntry>());
	private readonly entriesByModel = new WeakMap<TextModel, StandaloneModelEntry>();
	private readonly createEmitter = this._register(new Emitter<TextModel>());
	private readonly willDisposeEmitter = this._register(new Emitter<TextModel>());
	private readonly languageEmitter = this._register(new Emitter<StandaloneModelLanguageChangeEvent>());
	private modelIdentity = 1;

	readonly onDidCreateModel = this.createEmitter.event;
	readonly onWillDisposeModel = this.willDisposeEmitter.event;
	readonly onDidChangeModelLanguage = this.languageEmitter.event;

	createModel(value: string, languageId = "plaintext", resource = this.nextResource()): TextModel {
		if (typeof value !== "string") throw new TypeError("Standalone model value must be a string");
		const normalizedLanguageId = requireLanguageId(languageId);
		const key = resource.toString();
		if (this.entries.has(key)) throw new Error(`A standalone model already exists for '${key}'`);
		const model = new TextModel(value);
		try {
			const entry = new StandaloneModelEntry(model, resource, normalizedLanguageId, () => this.removeEntry(key, model));
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

	private entryForResource(resource: URI): StandaloneModelEntry | undefined {
		const key = resource.toString();
		for (const [candidate, entry] of this.entries) if (candidate === key) return entry;
		return undefined;
	}

	private requireEntry(model: TextModel): StandaloneModelEntry {
		const entry = this.entriesByModel.get(model);
		if (!entry) throw new ReferenceError("TextModel is not registered with the standalone model service");
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

class StandaloneModelEntry extends Disposable {
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
		throw new TypeError("Standalone model language id must be a non-empty string");
	}
	return languageId;
}
