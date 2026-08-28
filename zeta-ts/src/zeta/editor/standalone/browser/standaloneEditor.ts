import { Emitter, type Event } from "../../../base/common/event.js";
import { toDisposable } from "../../../base/common/lifecycle.js";
import { URI } from "../../../base/common/uri.js";
import { type EditorBrowserOptions } from "../../browser/editorBrowser.js";
import { TextModel } from "../../common/model/textModel.js";
import { type ModelLanguageChangeEvent } from "../../common/services/model.js";
import { type IStandaloneThemeData } from "../common/standaloneTheme.js";
import { StandaloneCodeEditor, type IStandaloneCodeEditor } from "./standaloneCodeEditor.js";
import { StandaloneServices, type StandaloneServiceOverrides } from "./standaloneServices.js";

type StandaloneEditorBrowserOptions = Omit<EditorBrowserOptions,
	"container" | "input" | "languageId" | "model" | "languageFeaturesService" |
	"languageConfigurationService" | "editorWorkerFactory" | "syntaxWorkerFactory" | "completionWorkerFactory" | "instantiationService"
>;

export interface IStandaloneEditorConstructionOptions extends StandaloneEditorBrowserOptions {
	readonly model?: TextModel;
	readonly value?: string;
	readonly languageId?: string;
	readonly resource?: URI;
	readonly label?: string;
	readonly readOnly?: boolean;
	readonly theme?: string;
	readonly autoDetectHighContrast?: boolean;
}

export interface IStandaloneEditorApi {
	readonly create: typeof create;
	readonly createModel: typeof createModel;
	readonly getModel: typeof getModel;
	readonly getModels: typeof getModels;
	readonly getModelResource: typeof getModelResource;
	readonly getModelLanguage: typeof getModelLanguage;
	readonly setModelLanguage: typeof setModelLanguage;
	readonly getEditors: typeof getEditors;
	readonly onDidCreateEditor: Event<IStandaloneCodeEditor>;
	readonly onDidCreateModel: Event<TextModel>;
	readonly onWillDisposeModel: Event<TextModel>;
	readonly onDidChangeModelLanguage: Event<ModelLanguageChangeEvent>;
	readonly defineTheme: typeof defineTheme;
	readonly setTheme: typeof setTheme;
}

const editors = new Set<IStandaloneCodeEditor>();
const createEditorEmitter = new Emitter<IStandaloneCodeEditor>();

export const onDidCreateEditor: Event<IStandaloneCodeEditor> = createEditorEmitter.event;
export const onDidCreateModel: Event<TextModel> = listener => StandaloneServices.get().modelService.onDidCreateModel(listener);
export const onWillDisposeModel: Event<TextModel> = listener => StandaloneServices.get().modelService.onWillDisposeModel(listener);
export const onDidChangeModelLanguage: Event<ModelLanguageChangeEvent> = listener => StandaloneServices.get().modelService.onDidChangeModelLanguage(listener);

/** Creates one browser editor. A supplied model must come from createModel(). */
export function create(
	domElement: HTMLElement,
	options: IStandaloneEditorConstructionOptions = {},
	overrides: StandaloneServiceOverrides = {},
): IStandaloneCodeEditor {
	if (!domElement || domElement.nodeType !== 1 || !domElement.ownerDocument) throw new TypeError("Standalone editor requires an HTML element");
	const services = StandaloneServices.initialize(overrides);
	const {
		model: suppliedModel,
		value,
		languageId,
		resource,
		label,
		readOnly,
		theme,
		autoDetectHighContrast,
		...browserOptions
	} = options;
	if (suppliedModel && (value !== undefined || languageId !== undefined || resource !== undefined)) {
		throw new TypeError("Standalone editor model cannot be combined with value, languageId, or resource");
	}
	if (theme !== undefined) services.themeService.setTheme(theme);
	if (autoDetectHighContrast !== undefined) services.themeService.setAutoDetectHighContrast(autoDetectHighContrast);
	const model = suppliedModel ?? services.modelService.createModel(value ?? "", languageId, resource);
	const ownsModel = suppliedModel === undefined;
	try {
		const modelResource = services.modelService.getModelResource(model);
		const editorOptions: EditorBrowserOptions = {
			...browserOptions,
			container: domElement,
			input: { resource: modelResource, label, readOnly },
			languageId: services.modelService.getModelLanguage(model),
			model,
			languageFeaturesService: services.languageFeaturesService,
			languageConfigurationService: services.languageConfigurationService,
			editorWorkerFactory: services.editorWorkerFactory,
			syntaxWorkerFactory: services.syntaxWorkerFactory,
			instantiationService: services.instantiationService,
		};
		const editor = services.completionWorkerFactory
			? new StandaloneCodeEditor({ ...editorOptions, completionWorkerFactory: services.completionWorkerFactory }, model, ownsModel, services.themeService)
			: new StandaloneCodeEditor(editorOptions, model, ownsModel, services.themeService);
		editors.add(editor);
		editor.registerEditorLifetime(toDisposable(() => editors.delete(editor)));
		createEditorEmitter.fire(editor);
		return editor;
	} catch (error) {
		if (ownsModel) model.dispose();
		throw error;
	}
}

export function createModel(value: string, languageId?: string, resource?: URI): TextModel {
	return StandaloneServices.get().modelService.createModel(value, languageId, resource);
}

export function getModel(resource: URI): TextModel | undefined {
	return StandaloneServices.get().modelService.getModel(resource);
}

export function getModels(): readonly TextModel[] {
	return StandaloneServices.get().modelService.getModels();
}

export function getModelResource(model: TextModel): URI {
	return StandaloneServices.get().modelService.getModelResource(model);
}

export function getModelLanguage(model: TextModel): string {
	return StandaloneServices.get().modelService.getModelLanguage(model);
}

export function setModelLanguage(model: TextModel, languageId: string): void {
	StandaloneServices.get().modelService.setModelLanguage(model, languageId);
}

export function getEditors(): readonly IStandaloneCodeEditor[] {
	return Object.freeze([...editors]);
}

export function defineTheme(themeId: string, themeData: IStandaloneThemeData): void {
	StandaloneServices.get().themeService.defineTheme(themeId, themeData);
}

export function setTheme(themeId: string): void {
	StandaloneServices.get().themeService.setTheme(themeId);
}

export function createStandaloneEditorApi(): IStandaloneEditorApi {
	return Object.freeze({
		create,
		createModel,
		getModel,
		getModels,
		getModelResource,
		getModelLanguage,
		setModelLanguage,
		getEditors,
		onDidCreateEditor,
		onDidCreateModel,
		onWillDisposeModel,
		onDidChangeModelLanguage,
		defineTheme,
		setTheme,
	});
}

export type { IStandaloneCodeEditor } from './standaloneCodeEditor.js';
