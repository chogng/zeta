import { Emitter, type Event } from "../../../base/common/event.js";
import { toDisposable } from "../../../base/common/lifecycle.js";
import { URI } from "../../../base/common/uri.js";
import { ContentWidgetPositionPreference, type EditorBrowserOptions, OverlayWidgetPositionPreference } from "../../browser/editorBrowser.js";
import { PositionAffinity } from "../../common/model.js";
import { TextModel } from "../../common/model/textModel.js";
import { type NamedEditorThemeData } from "../common/namedEditorTheme.js";
import { StandaloneEditorInstance, type IStandaloneEditorInstance } from "./standaloneEditorInstance.js";
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
	readonly ContentWidgetPositionPreference: typeof ContentWidgetPositionPreference;
	readonly OverlayWidgetPositionPreference: typeof OverlayWidgetPositionPreference;
	readonly PositionAffinity: typeof PositionAffinity;
	readonly create: typeof create;
	readonly createModel: typeof createModel;
	readonly getModel: typeof getModel;
	readonly getModels: typeof getModels;
	readonly setModelLanguage: typeof setModelLanguage;
	readonly getEditors: typeof getEditors;
	readonly onDidCreateEditor: Event<IStandaloneEditorInstance>;
	readonly onDidCreateModel: Event<TextModel>;
	readonly onWillDisposeModel: Event<TextModel>;
	readonly onDidChangeModelLanguage: Event<{ readonly model: TextModel; readonly oldLanguage: string }>;
	readonly defineNamedTheme: typeof defineNamedTheme;
	readonly setTheme: typeof setTheme;
}

const editors = new Set<IStandaloneEditorInstance>();
const createEditorEmitter = new Emitter<IStandaloneEditorInstance>();

export const onDidCreateEditor: Event<IStandaloneEditorInstance> = createEditorEmitter.event;
export const onDidCreateModel: Event<TextModel> = listener => StandaloneServices.get().modelService.onModelAdded(listener);
export const onWillDisposeModel: Event<TextModel> = listener => StandaloneServices.get().modelService.onModelRemoved(listener);
export const onDidChangeModelLanguage: Event<{ readonly model: TextModel; readonly oldLanguage: string }> = listener =>
	StandaloneServices.get().modelService.onModelLanguageChanged(event => listener({ model: event.model, oldLanguage: event.oldLanguageId }));

/** Creates one browser editor. A supplied model must come from createModel(). */
export function create(
	domElement: HTMLElement,
	options: IStandaloneEditorConstructionOptions = {},
	overrides: StandaloneServiceOverrides = {},
): IStandaloneEditorInstance {
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
	const model = suppliedModel ?? services.modelService.createModel(value ?? "", services.languageService.createById(languageId), resource);
	const ownsModel = suppliedModel === undefined;
	try {
		if (services.modelService.getModel(model.uri) !== model) throw new ReferenceError('Standalone editor model is not registered with the model service');
		const editorOptions: EditorBrowserOptions = {
			...browserOptions,
			container: domElement,
			input: { resource: model.uri, label, readOnly },
			languageId: model.getLanguageId(),
			model,
			languageFeaturesService: services.languageFeaturesService,
			languageConfigurationService: services.languageConfigurationService,
			editorWorkerFactory: services.editorWorkerFactory,
			syntaxWorkerFactory: services.syntaxWorkerFactory,
			instantiationService: services.instantiationService,
			codeEditorService: services.codeEditorService,
		};
		const editor = services.completionWorkerFactory
			? new StandaloneEditorInstance({ ...editorOptions, completionWorkerFactory: services.completionWorkerFactory }, model, ownsModel, services.themeService)
			: new StandaloneEditorInstance(editorOptions, model, ownsModel, services.themeService);
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
	const services = StandaloneServices.get();
	return services.modelService.createModel(value, services.languageService.createById(languageId), resource);
}

export function getModel(resource: URI): TextModel | null {
	return StandaloneServices.get().modelService.getModel(resource);
}

export function getModels(): TextModel[] {
	return StandaloneServices.get().modelService.getModels();
}

export function setModelLanguage(model: TextModel, languageId: string): void {
	model.setLanguage(languageId);
}

export function getEditors(): readonly IStandaloneEditorInstance[] {
	return Object.freeze([...editors]);
}

export function defineNamedTheme(themeId: string, themeData: NamedEditorThemeData): void {
	StandaloneServices.get().themeService.defineNamedTheme(themeId, themeData);
}

export function setTheme(themeId: string): void {
	StandaloneServices.get().themeService.setTheme(themeId);
}

export function createStandaloneEditorApi(): IStandaloneEditorApi {
	return Object.freeze({
		ContentWidgetPositionPreference,
		OverlayWidgetPositionPreference,
		PositionAffinity,
		create,
		createModel,
		getModel,
		getModels,
		setModelLanguage,
		getEditors,
		onDidCreateEditor,
		onDidCreateModel,
		onWillDisposeModel,
		onDidChangeModelLanguage,
		defineNamedTheme,
		setTheme,
	});
}

export type { IStandaloneEditorInstance } from './standaloneEditorInstance.js';
