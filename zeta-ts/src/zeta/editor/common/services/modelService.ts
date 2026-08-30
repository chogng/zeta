import { Emitter } from "../../../base/common/event.js";
import { Disposable, DisposableMap } from "../../../base/common/lifecycle.js";
import { URI } from "../../../base/common/uri.js";
import { TextModel } from "../model/textModel.js";
import type { ILanguageSelection } from '../languages/language.js';
import type { IModelService } from "./model.js";

/** Resource and language registry for caller-owned TextModels. */
export class ModelService extends Disposable implements IModelService {
	private readonly entries = this._register(new DisposableMap<string, ModelEntry>());
	private readonly entriesByModel = new WeakMap<TextModel, ModelEntry>();
	private readonly createEmitter = this._register(new Emitter<TextModel>());
	private readonly willDisposeEmitter = this._register(new Emitter<TextModel>());
	private readonly languageEmitter = this._register(new Emitter<{ readonly model: TextModel; readonly oldLanguageId: string }>());
	readonly _serviceBrand: undefined;

	readonly onModelAdded = this.createEmitter.event;
	readonly onModelRemoved = this.willDisposeEmitter.event;
	readonly onModelLanguageChanged = this.languageEmitter.event;

	createModel(value: string, languageSelection: ILanguageSelection | null, resource?: URI, isForSimpleWidget = false): TextModel {
		if (typeof value !== "string") throw new TypeError("Model value must be a string");
		if (resource && this.entries.has(resource.toString())) throw new Error(`A model already exists for '${resource.toString()}'`);
		const model = new TextModel(value, { resource, languageId: languageSelection?.languageId, isForSimpleWidget });
		if (languageSelection) model.setLanguage(languageSelection);
		const key = model.uri.toString();
		if (this.entries.has(key)) {
			model.dispose();
			throw new Error(`A model already exists for '${key}'`);
		}
		try {
			const entry = new ModelEntry(
				model,
				() => this.removeEntry(key, model),
				oldLanguageId => this.languageEmitter.fire(Object.freeze({ model, oldLanguageId })),
			);
			this.entries.set(key, entry);
			this.entriesByModel.set(model, entry);
			this.createEmitter.fire(model);
			return model;
		} catch (error) {
			model.dispose();
			throw error;
		}
	}

	updateModel(model: TextModel, value: string): void {
		this.requireEntry(model);
		model.reset(value);
	}

	destroyModel(resource: URI): void {
		this.entryForResource(resource)?.model.dispose();
	}

	getModel(resource: URI): TextModel | null {
		return this.entryForResource(resource)?.model ?? null;
	}

	getModels(): TextModel[] {
		return [...this.entries].map(([, entry]) => entry.model);
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
		if (!entry || entry.model.uri.toString() !== key) return;
		this.willDisposeEmitter.fire(model);
		this.entriesByModel.delete(model);
		this.entries.deleteAndDispose(key);
	}
}

class ModelEntry extends Disposable {
	constructor(
		readonly model: TextModel,
		onWillDispose: () => void,
		onDidChangeLanguage: (oldLanguageId: string) => void,
	) {
		super();
		this._register(model.onWillDispose(onWillDispose));
		this._register(model.onDidChangeLanguage(event => onDidChangeLanguage(event.oldLanguage)));
	}
}
