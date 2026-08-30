import { Disposable } from "../../../base/common/lifecycle.js";
import { ServiceContainer } from "../../../platform/instantiation/common/instantiation.js";
import { IThemeService } from "../../../platform/theme/common/themeService.js";
import { createEditorBrowserServices } from '../../browser/services/contribution.js';
import { type ICodeEditorService } from '../../browser/services/codeEditorService.js';
import { type LanguageCompletionWorkerFactory } from "../../common/languages/completion/languageCompletionService.js";
import { type SyntaxWorkerFactory } from "../../common/languages/syntax/syntaxService.js";
import { type VersionedEditorWorkerFactory } from "../../browser/services/versionedEditorWorkerClient.js";
import { registerBuiltinLanguageConfigurations } from "../../common/languages/languageBuiltinConfigurations.js";
import { registerBuiltinLanguageDescriptions } from "../../common/languages/languageBuiltinDescriptions.js";
import { ILanguageFeaturesService } from '../../common/services/languageFeatures.js';
import { LanguageFeaturesService } from '../../common/services/languageFeaturesService.js';
import { ILanguageService, type IZetaLanguageService } from '../../common/languages/language.js';
import { LanguageService } from '../../common/services/languageService.js';
import { IComposableLanguageConfigurationService, ComposableLanguageConfigurationService } from '../../common/languages/ownedLanguageConfigurationContributions.js';
import { IModelService } from '../../common/services/model.js';
import { ModelService } from '../../common/services/modelService.js';
import { type INamedEditorThemeService } from "../common/namedEditorTheme.js";
import { NamedEditorThemeService } from "./namedEditorThemeService.js";

export interface StandaloneServiceOverrides {
	readonly languageService?: IZetaLanguageService;
	readonly languageConfigurationService?: IComposableLanguageConfigurationService;
	readonly languageFeaturesService?: ILanguageFeaturesService;
	readonly editorWorkerFactory?: VersionedEditorWorkerFactory;
	readonly syntaxWorkerFactory?: SyntaxWorkerFactory;
	/** Explicit Worker authority that replaces the local completion provider registry. */
	readonly completionWorkerFactory?: LanguageCompletionWorkerFactory;
}

export class StandaloneServiceCollection extends Disposable {
	readonly instantiationService: ServiceContainer;
	readonly modelService: IModelService;
	readonly languageService: IZetaLanguageService;
	readonly languageConfigurationService: IComposableLanguageConfigurationService;
	readonly languageFeaturesService: ILanguageFeaturesService;
	readonly themeService: INamedEditorThemeService;
	readonly syntaxWorkerFactory: SyntaxWorkerFactory;
	readonly editorWorkerFactory: VersionedEditorWorkerFactory;
	readonly completionWorkerFactory: LanguageCompletionWorkerFactory | undefined;
	readonly codeEditorService: ICodeEditorService;

	constructor(overrides: StandaloneServiceOverrides) {
		super();
		const instantiationService = this.instantiationService = this._register(new ServiceContainer());
		if (overrides.languageFeaturesService && !overrides.languageConfigurationService) throw new TypeError("Standalone language feature overrides require a language configuration service");
		if (overrides.languageService) instantiationService.registerInstance(ILanguageService, overrides.languageService);
		else instantiationService.registerSingleton(ILanguageService, () => new LanguageService());
		if (overrides.languageConfigurationService) instantiationService.registerInstance(IComposableLanguageConfigurationService, overrides.languageConfigurationService);
		else instantiationService.registerSingleton(IComposableLanguageConfigurationService, () => new ComposableLanguageConfigurationService());
		if (overrides.languageFeaturesService) instantiationService.registerInstance(ILanguageFeaturesService, overrides.languageFeaturesService);
		else instantiationService.registerSingleton(ILanguageFeaturesService, accessor => new LanguageFeaturesService(accessor.get(IComposableLanguageConfigurationService)));
		instantiationService.registerSingleton(IThemeService, () => new NamedEditorThemeService(window));
		this.themeService = instantiationService.get(IThemeService) as INamedEditorThemeService;
		instantiationService.registerSingleton(IModelService, () => new ModelService());
		this.modelService = instantiationService.get(IModelService);
		this.languageService = instantiationService.get(ILanguageService);
		this.languageConfigurationService = instantiationService.get(IComposableLanguageConfigurationService);
		this.languageFeaturesService = instantiationService.get(ILanguageFeaturesService);
		if (!overrides.languageService) this._register(registerBuiltinLanguageDescriptions(this.languageService.languages));
		if (!overrides.languageConfigurationService) this._register(registerBuiltinLanguageConfigurations(this.languageConfigurationService.configurations));
		const browserServices = createEditorBrowserServices();
		this._register(browserServices.codeEditors);
		this.codeEditorService = browserServices.codeEditors;
		const workers = browserServices.workers;
		this.editorWorkerFactory = overrides.editorWorkerFactory ?? workers.editorWorkerFactory;
		this.syntaxWorkerFactory = overrides.syntaxWorkerFactory ?? workers.syntaxWorkerFactory;
		this.completionWorkerFactory = overrides.completionWorkerFactory;
	}
}

let services: StandaloneServiceCollection | undefined;

/** One browser-window service scope. The first editor may provide service overrides. */
export namespace StandaloneServices {
	export function initialize(overrides: StandaloneServiceOverrides = {}): StandaloneServiceCollection {
		if (services) {
			if (Object.keys(overrides).length > 0) throw new Error("Standalone services are already initialized");
			return services;
		}
		services = new StandaloneServiceCollection(overrides);
		return services;
	}

	export function get(): StandaloneServiceCollection {
		return initialize();
	}
}
