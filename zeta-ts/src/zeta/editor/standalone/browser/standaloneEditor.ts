import { Emitter } from "../../../base/common/event.js";
import { toDisposable, type IDisposable } from "../../../base/common/lifecycle.js";
import { URI } from "../../../base/common/uri.js";
import { ContentWidgetPositionPreference, OverlayWidgetPositionPreference } from "../../browser/editorBrowser.js";
import { type ConfiguredCodeEditorOptions } from '../../browser/configuredCodeEditor.js';
import { PositionAffinity } from "../../common/model.js";
import { TextModel } from "../../common/model/textModel.js";
import { type NamedEditorThemeData } from "../common/namedEditorTheme.js";
import { StandaloneEditorInstance, type IStandaloneEditorInstance } from "./standaloneEditorInstance.js";
import { StandaloneServices, type StandaloneServiceOverrides } from "./standaloneServices.js";

type StandaloneConfiguredCodeEditorOptions = Omit<ConfiguredCodeEditorOptions,
	"container" | "input" | "languageId" | "model" | "languageFeaturesService" |
	"languageConfigurationService" | "editorWorkerFactory" | "syntaxWorkerFactory" | "completionWorkerFactory" | "instantiationService"
>;

export interface IStandaloneEditorConstructionOptions extends StandaloneConfiguredCodeEditorOptions {
	readonly model?: TextModel;
	readonly value?: string;
	readonly language?: string;
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
	readonly onDidCreateEditor: typeof onDidCreateEditor;
	readonly onDidCreateModel: typeof onDidCreateModel;
	readonly onWillDisposeModel: typeof onWillDisposeModel;
	readonly onDidChangeModelLanguage: typeof onDidChangeModelLanguage;
	readonly defineNamedTheme: typeof defineNamedTheme;
	readonly setTheme: typeof setTheme;
}

const editors = new Set<IStandaloneEditorInstance>();
const createEditorEmitter = new Emitter<IStandaloneEditorInstance>();
const contentWidgetPositionPreference = Object.freeze({
	EXACT: ContentWidgetPositionPreference.EXACT,
	ABOVE: ContentWidgetPositionPreference.ABOVE,
	BELOW: ContentWidgetPositionPreference.BELOW,
});
const overlayWidgetPositionPreference = Object.freeze({
	TOP_RIGHT_CORNER: OverlayWidgetPositionPreference.TOP_RIGHT_CORNER,
	BOTTOM_RIGHT_CORNER: OverlayWidgetPositionPreference.BOTTOM_RIGHT_CORNER,
	TOP_CENTER: OverlayWidgetPositionPreference.TOP_CENTER,
});

export function onDidCreateEditor(listener: (codeEditor: IStandaloneEditorInstance) => void): IDisposable {
	return createEditorEmitter.event(listener);
}

export function onDidCreateModel(listener: (model: TextModel) => void): IDisposable {
	return StandaloneServices.get().modelService.onModelAdded(listener);
}

export function onWillDisposeModel(listener: (model: TextModel) => void): IDisposable {
	return StandaloneServices.get().modelService.onModelRemoved(listener);
}

export function onDidChangeModelLanguage(listener: (event: { readonly model: TextModel; readonly oldLanguage: string }) => void): IDisposable {
	return StandaloneServices.get().modelService.onModelLanguageChanged(event => listener({ model: event.model, oldLanguage: event.oldLanguageId }));
}

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
		language,
		resource,
		label,
		readOnly,
		theme,
		autoDetectHighContrast,
		...browserOptions
	} = options;
	if (suppliedModel && (value !== undefined || language !== undefined || resource !== undefined)) {
		throw new TypeError("Standalone editor model cannot be combined with value, language, or resource");
	}
	if (theme !== undefined) services.themeService.setTheme(theme);
	if (autoDetectHighContrast !== undefined) services.themeService.setAutoDetectHighContrast(autoDetectHighContrast);
	const languageId = services.languageService.getLanguageIdByMimeType(language) ?? language;
	const model = suppliedModel ?? services.modelService.createModel(value ?? "", services.languageService.createById(languageId), resource);
	const ownsModel = suppliedModel === undefined;
	try {
		if (services.modelService.getModel(model.uri) !== model) throw new ReferenceError('Standalone editor model is not registered with the model service');
		const editorOptions: ConfiguredCodeEditorOptions = {
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

export function createModel(value: string, language?: string, uri?: URI): TextModel {
	const services = StandaloneServices.get();
	const languageId = services.languageService.getLanguageIdByMimeType(language) ?? language;
	return services.modelService.createModel(value, services.languageService.createById(languageId), uri);
}

export function getModel(uri: URI): TextModel | null {
	return StandaloneServices.get().modelService.getModel(uri);
}

export function getModels(): TextModel[] {
	return StandaloneServices.get().modelService.getModels();
}

export function setModelLanguage(model: TextModel, mimeTypeOrLanguageId: string): void {
	const languageService = StandaloneServices.get().languageService;
	const languageId = languageService.getLanguageIdByMimeType(mimeTypeOrLanguageId) ?? (mimeTypeOrLanguageId || 'plaintext');
	model.setLanguage(languageService.createById(languageId));
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
		ContentWidgetPositionPreference: contentWidgetPositionPreference,
		OverlayWidgetPositionPreference: overlayWidgetPositionPreference,
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
