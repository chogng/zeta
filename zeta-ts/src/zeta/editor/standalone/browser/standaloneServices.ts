import { Disposable } from "../../../base/common/lifecycle.js";
import { ServiceContainer } from "../../../platform/instantiation/common/instantiation.js";
import { IThemeService } from "../../../platform/theme/common/themeService.js";
import { IConfigurationService } from '../../../platform/configuration/common/configuration.js';
import { InMemoryConfigurationService } from '../../../platform/configuration/common/inMemoryConfigurationService.js';
import { createEditorBrowserServices } from '../../browser/services/contribution.js';
import { type IWidgetCodeEditorRegistry } from '../../browser/services/codeEditorService.js';
import { type LanguageCompletionWorkerFactory } from "../../common/languages/completion/languageCompletionService.js";
import { type SyntaxWorkerFactory } from "../../common/languages/syntax/syntaxService.js";
import { type VersionedEditorWorkerFactory } from "../../browser/services/editorWorkerService.js";
import { registerBuiltinLanguageConfigurations } from "../../common/languages/languageBuiltinConfigurations.js";
import { registerBuiltinLanguageDescriptions } from "../../common/languages/languageBuiltinDescriptions.js";
import { ILanguageFeaturesService } from '../../common/services/languageFeatures.js';
import { LanguageFeaturesService } from '../../common/services/languageFeaturesService.js';
import { ILanguageService, type IZetaLanguageService } from '../../common/languages/language.js';
import { LanguageService } from '../../common/services/languageService.js';
import { ILanguageConfigurationService, LanguageConfigurationService } from '../../common/languages/languageConfigurationRegistry.js';
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
	readonly languageConfigurationService?: ILanguageConfigurationService;
	readonly languageFeaturesService?: ILanguageFeaturesService;
	readonly editorWorkerFactory?: VersionedEditorWorkerFactory;
	readonly syntaxWorkerFactory?: SyntaxWorkerFactory;
	/** Explicit Worker authority that replaces the local completion provider registry. */
	readonly completionWorkerFactory?: LanguageCompletionWorkerFactory;
}

export class StandaloneServiceCollection extends Disposable {
	readonly instantiationService: ServiceContainer;
	readonly modelService: ModelService;
	readonly languageService: IZetaLanguageService;
	readonly languageConfigurationService: ILanguageConfigurationService;
	readonly languageFeaturesService: ILanguageFeaturesService;
	readonly themeService: INamedEditorThemeService;
	readonly syntaxWorkerFactory: SyntaxWorkerFactory;
	readonly editorWorkerFactory: VersionedEditorWorkerFactory;
	readonly completionWorkerFactory: LanguageCompletionWorkerFactory | undefined;
	readonly codeEditorService: IWidgetCodeEditorRegistry;

	constructor(overrides: StandaloneServiceOverrides) {
		super();
		const instantiationService = this.instantiationService = this._register(new ServiceContainer());
		const browserServices = createEditorBrowserServices();
		this._register(browserServices.codeEditors);
		this.codeEditorService = browserServices.codeEditors;
		const workers = browserServices.workers;
		this.editorWorkerFactory = overrides.editorWorkerFactory ?? workers.editorWorkerFactory;
		this.syntaxWorkerFactory = overrides.syntaxWorkerFactory ?? workers.syntaxWorkerFactory;
		this.completionWorkerFactory = overrides.completionWorkerFactory;
		const configurationService = this._register(new InMemoryConfigurationService());
		instantiationService.registerInstance(IConfigurationService, configurationService);
		instantiationService.registerSingleton(ITextResourcePropertiesService, accessor => new StandaloneResourcePropertiesService(accessor.get(IConfigurationService)));
		if (overrides.languageFeaturesService && !overrides.languageConfigurationService) throw new TypeError("Standalone language feature overrides require a language configuration service");
		if (overrides.languageService) instantiationService.registerInstance(ILanguageService, overrides.languageService);
		else instantiationService.registerSingleton(ILanguageService, () => new LanguageService());
		if (overrides.languageConfigurationService) instantiationService.registerInstance(ILanguageConfigurationService, overrides.languageConfigurationService);
		else instantiationService.registerSingleton(ILanguageConfigurationService, accessor => new LanguageConfigurationService(
			accessor.get(IConfigurationService),
			accessor.get(ILanguageService),
		));
		if (overrides.languageFeaturesService) instantiationService.registerInstance(ILanguageFeaturesService, overrides.languageFeaturesService);
		else instantiationService.registerSingleton(ILanguageFeaturesService, accessor => new LanguageFeaturesService(accessor.get(ILanguageConfigurationService)));
		instantiationService.registerSingleton(IThemeService, () => new NamedEditorThemeService(window));
		this.themeService = instantiationService.get(IThemeService) as INamedEditorThemeService;
		instantiationService.registerSingleton(IModelService, accessor => new ModelService(
			accessor.get(IConfigurationService),
			accessor.get(ITextResourcePropertiesService),
			accessor.get(ILanguageService),
			accessor.get(ILanguageFeaturesService),
			accessor.get(ILanguageConfigurationService),
			{ syntaxService: { workerFactory: this.syntaxWorkerFactory } },
		));
		this.modelService = instantiationService.get(IModelService) as ModelService;
		this.languageService = instantiationService.get(ILanguageService);
		this.languageConfigurationService = instantiationService.get(ILanguageConfigurationService);
		this.languageFeaturesService = instantiationService.get(ILanguageFeaturesService);
		if (!overrides.languageService) this._register(registerBuiltinLanguageDescriptions(this.languageService.languages));
		if (!overrides.languageConfigurationService) this._register(registerBuiltinLanguageConfigurations(this.languageConfigurationService));
	}
}

class StandaloneResourcePropertiesService implements ITextResourcePropertiesServiceContract {
	readonly _serviceBrand: undefined;

	constructor(private readonly configurationService: IConfigurationService) {}

	getEOL(resource: URI, language?: string): string {
		const eol = this.configurationService.getValue<'auto' | '\n' | '\r\n'>(EditorModelConfiguration.filesEol, { overrideIdentifier: language, resource });
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
