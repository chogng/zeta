import { Disposable } from "../../../base/common/lifecycle.js";
import { ServiceContainer } from "../../../platform/instantiation/common/instantiation.js";
import { IThemeService } from "../../../platform/theme/common/themeService.js";
import { IConfigurationService } from '../../../platform/configuration/common/configurationService.js';
import { InMemoryConfigurationService } from '../../../platform/configuration/common/inMemoryConfigurationService.js';
import { createEditorBrowserServices } from '../../browser/services/contribution.js';
import { type IWidgetCodeEditorRegistry } from '../../browser/services/codeEditorService.js';
import { type LanguageCompletionWorkerFactory } from "../../common/languages/completion/languageCompletionService.js";
import { type SyntaxWorkerFactory } from "../../common/languages/syntax/syntaxService.js";
import { type VersionedEditorWorkerFactory } from "../../browser/services/editorWorkerService.js";
import { registerBuiltinLanguageConfigurations } from "../../common/languages/languageBuiltinConfigurations.js";
import { registerBuiltinLanguageDescriptions } from "../../common/languages/languageBuiltinDescriptions.js";
import { IEditorLanguageFeaturesService } from '../../common/services/languageFeatures.js';
import { EditorLanguageFeaturesService } from '../../common/services/languageFeaturesService.js';
import { ILanguageService, type IZetaLanguageService } from '../../common/languages/language.js';
import { LanguageService } from '../../common/services/languageService.js';
import { IComposableLanguageConfigurationService, ComposableLanguageConfigurationService } from '../../common/languages/ownedLanguageConfigurationContributions.js';
import { IModelService } from '../../common/services/model.js';
import { ModelService } from '../../common/services/modelService.js';
import { ITextResourcePropertiesService, type ITextResourcePropertiesService as ITextResourcePropertiesServiceContract } from '../../common/services/textResourceConfiguration.js';
import { EditorModelConfiguration } from '../../common/config/editorModelConfiguration.js';
import { isLinux, isMacintosh } from '../../../base/common/platform.js';
import type { URI } from '../../../base/common/uri.js';
import { type INamedEditorThemeService } from "../common/namedEditorTheme.js";
import { NamedEditorThemeService } from "./namedEditorThemeService.js";

export interface StandaloneServiceOverrides {
	readonly languageService?: IZetaLanguageService;
	readonly languageConfigurationService?: IComposableLanguageConfigurationService;
	readonly languageFeaturesService?: IEditorLanguageFeaturesService;
	readonly editorWorkerFactory?: VersionedEditorWorkerFactory;
	readonly syntaxWorkerFactory?: SyntaxWorkerFactory;
	/** Explicit Worker authority that replaces the local completion provider registry. */
	readonly completionWorkerFactory?: LanguageCompletionWorkerFactory;
}

export class StandaloneServiceCollection extends Disposable {
	readonly instantiationService: ServiceContainer;
	readonly modelService: ModelService;
	readonly languageService: IZetaLanguageService;
	readonly languageConfigurationService: IComposableLanguageConfigurationService;
	readonly languageFeaturesService: IEditorLanguageFeaturesService;
	readonly themeService: INamedEditorThemeService;
	readonly syntaxWorkerFactory: SyntaxWorkerFactory;
	readonly editorWorkerFactory: VersionedEditorWorkerFactory;
	readonly completionWorkerFactory: LanguageCompletionWorkerFactory | undefined;
	readonly codeEditorService: IWidgetCodeEditorRegistry;

	constructor(overrides: StandaloneServiceOverrides) {
		super();
		const instantiationService = this.instantiationService = this._register(new ServiceContainer());
		const configurationService = this._register(new InMemoryConfigurationService());
		instantiationService.registerInstance(IConfigurationService, configurationService);
		instantiationService.registerSingleton(ITextResourcePropertiesService, accessor => new StandaloneResourcePropertiesService(accessor.get(IConfigurationService)));
		if (overrides.languageFeaturesService && !overrides.languageConfigurationService) throw new TypeError("Standalone language feature overrides require a language configuration service");
		if (overrides.languageService) instantiationService.registerInstance(ILanguageService, overrides.languageService);
		else instantiationService.registerSingleton(ILanguageService, () => new LanguageService());
		if (overrides.languageConfigurationService) instantiationService.registerInstance(IComposableLanguageConfigurationService, overrides.languageConfigurationService);
		else instantiationService.registerSingleton(IComposableLanguageConfigurationService, () => new ComposableLanguageConfigurationService());
		if (overrides.languageFeaturesService) instantiationService.registerInstance(IEditorLanguageFeaturesService, overrides.languageFeaturesService);
		else instantiationService.registerSingleton(IEditorLanguageFeaturesService, accessor => new EditorLanguageFeaturesService(accessor.get(IComposableLanguageConfigurationService)));
		instantiationService.registerSingleton(IThemeService, () => new NamedEditorThemeService(window));
		this.themeService = instantiationService.get(IThemeService) as INamedEditorThemeService;
		instantiationService.registerSingleton(IModelService, accessor => new ModelService(
			accessor.get(IConfigurationService),
			accessor.get(ITextResourcePropertiesService),
		));
		this.modelService = instantiationService.get(IModelService) as ModelService;
		this.languageService = instantiationService.get(ILanguageService);
		this.languageConfigurationService = instantiationService.get(IComposableLanguageConfigurationService);
		this.languageFeaturesService = instantiationService.get(IEditorLanguageFeaturesService);
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

class StandaloneResourcePropertiesService implements ITextResourcePropertiesServiceContract {
	readonly _serviceBrand: undefined;

	constructor(private readonly configurationService: IConfigurationService) {}

	getEOL(resource: URI, language?: string): string {
		const eol = this.configurationService.getValue(EditorModelConfiguration.filesEol, { overrideIdentifier: language, resource });
		return eol === 'auto' ? (isLinux || isMacintosh ? '\n' : '\r\n') : eol;
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
