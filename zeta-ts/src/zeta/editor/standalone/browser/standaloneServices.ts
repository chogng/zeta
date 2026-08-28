import { Disposable } from "../../../base/common/lifecycle.js";
import { ServiceContainer } from "../../../platform/instantiation/common/instantiation.js";
import { IThemeService } from "../../../platform/theme/common/themeService.js";
import { EditorWorkerService } from "../../browser/services/editorWorkerService.js";
import { type LanguageCompletionWorkerFactory } from "../../common/languages/completion/languageCompletionService.js";
import { type SyntaxWorkerFactory } from "../../common/languages/syntax/syntaxService.js";
import { type EditorWorkerFactory } from "../../common/services/editorWorker.js";
import { registerBuiltinLanguageConfigurations } from "../../common/languages/languageBuiltinConfigurations.js";
import { registerBuiltinLanguageDescriptions } from "../../common/languages/languageBuiltinDescriptions.js";
import { ILanguageFeaturesService } from '../../common/services/languageFeatures.js';
import { LanguageFeaturesService } from '../../common/services/languageFeaturesService.js';
import { ILanguageService, LanguageService } from '../../common/services/languageService.js';
import { ILanguageConfigurationService, LanguageConfigurationService } from '../../common/services/languageConfigurationService.js';
import { IModelService } from '../../common/services/model.js';
import { ModelService } from '../../common/services/modelService.js';
import { type IStandaloneThemeService } from "../common/standaloneTheme.js";
import { StandaloneThemeService } from "./standaloneThemeService.js";

export interface StandaloneServiceOverrides {
	readonly languageService?: ILanguageService;
	readonly languageConfigurationService?: ILanguageConfigurationService;
	readonly languageFeaturesService?: ILanguageFeaturesService;
	readonly editorWorkerFactory?: EditorWorkerFactory;
	readonly syntaxWorkerFactory?: SyntaxWorkerFactory;
	/** Explicit Worker authority that replaces the local completion provider registry. */
	readonly completionWorkerFactory?: LanguageCompletionWorkerFactory;
}

export class StandaloneServiceCollection extends Disposable {
	readonly instantiationService: ServiceContainer;
	readonly modelService: IModelService;
	readonly languageService: ILanguageService;
	readonly languageConfigurationService: ILanguageConfigurationService;
	readonly languageFeaturesService: ILanguageFeaturesService;
	readonly themeService: IStandaloneThemeService;
	readonly syntaxWorkerFactory: SyntaxWorkerFactory;
	readonly editorWorkerFactory: EditorWorkerFactory;
	readonly completionWorkerFactory: LanguageCompletionWorkerFactory | undefined;

	constructor(overrides: StandaloneServiceOverrides) {
		super();
		const instantiationService = this.instantiationService = this._register(new ServiceContainer());
		if (overrides.languageFeaturesService && !overrides.languageConfigurationService) throw new TypeError("Standalone language feature overrides require a language configuration service");
		if (overrides.languageService) instantiationService.registerInstance(ILanguageService, overrides.languageService);
		else instantiationService.registerSingleton(ILanguageService, () => new LanguageService());
		if (overrides.languageConfigurationService) instantiationService.registerInstance(ILanguageConfigurationService, overrides.languageConfigurationService);
		else instantiationService.registerSingleton(ILanguageConfigurationService, () => new LanguageConfigurationService());
		if (overrides.languageFeaturesService) instantiationService.registerInstance(ILanguageFeaturesService, overrides.languageFeaturesService);
		else instantiationService.registerSingleton(ILanguageFeaturesService, accessor => new LanguageFeaturesService(accessor.get(ILanguageConfigurationService)));
		instantiationService.registerSingleton(IThemeService, () => new StandaloneThemeService(window));
		this.themeService = instantiationService.get(IThemeService) as IStandaloneThemeService;
		instantiationService.registerSingleton(IModelService, () => new ModelService());
		this.modelService = instantiationService.get(IModelService);
		this.languageService = instantiationService.get(ILanguageService);
		this.languageConfigurationService = instantiationService.get(ILanguageConfigurationService);
		this.languageFeaturesService = instantiationService.get(ILanguageFeaturesService);
		if (!overrides.languageService) this._register(registerBuiltinLanguageDescriptions(this.languageService.languages));
		if (!overrides.languageConfigurationService) this._register(registerBuiltinLanguageConfigurations(this.languageConfigurationService.configurations));
		const workers = new EditorWorkerService();
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
