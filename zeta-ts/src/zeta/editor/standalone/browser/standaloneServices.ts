import { Disposable } from "../../../base/common/lifecycle.js";
import { createServiceIdentifier, ServiceContainer, type ServiceIdentifier } from "../../../platform/instantiation/common/instantiation.js";
import { IThemeService } from "../../../platform/theme/common/themeService.js";
import { createCompletionWorkerFactory } from "../../browser/language/languageCompletionWorkerClient.js";
import { createSyntaxWorkerFactory } from "../../browser/language/syntaxWorkerClient.js";
import { type LanguageCompletionWorkerFactory } from "../../common/languages/completion/languageCompletionService.js";
import { type SyntaxWorkerFactory } from "../../common/languages/syntax/syntaxService.js";
import { LanguageFeaturesService, type ILanguageFeaturesService } from "../../common/services/languageService.js";
import { type IStandaloneThemeService } from "../common/standaloneTheme.js";
import { IStandaloneModelService, StandaloneModelService } from "./standaloneModelService.js";
import { StandaloneThemeService } from "./standaloneThemeService.js";

const IStandaloneLanguageFeaturesService: ServiceIdentifier<ILanguageFeaturesService> = createServiceIdentifier<ILanguageFeaturesService>("standaloneLanguageFeaturesService");

export interface StandaloneServiceOverrides {
	readonly languageFeaturesService?: ILanguageFeaturesService;
	readonly syntaxWorkerFactory?: SyntaxWorkerFactory;
	readonly completionWorkerFactory?: LanguageCompletionWorkerFactory;
}

export class StandaloneServiceCollection extends Disposable {
	readonly instantiationService: ServiceContainer;
	readonly modelService: IStandaloneModelService;
	readonly languageFeaturesService: ILanguageFeaturesService;
	readonly themeService: IStandaloneThemeService;
	readonly syntaxWorkerFactory: SyntaxWorkerFactory;
	readonly completionWorkerFactory: LanguageCompletionWorkerFactory;

	constructor(overrides: StandaloneServiceOverrides) {
		super();
		const instantiationService = this.instantiationService = this._register(new ServiceContainer());
		if (overrides.languageFeaturesService) instantiationService.registerInstance(IStandaloneLanguageFeaturesService, overrides.languageFeaturesService);
		else instantiationService.registerSingleton(IStandaloneLanguageFeaturesService, () => new LanguageFeaturesService());
		instantiationService.registerSingleton(IThemeService, () => new StandaloneThemeService(window));
		this.themeService = instantiationService.get(IThemeService) as IStandaloneThemeService;
		instantiationService.registerSingleton(IStandaloneModelService, () => new StandaloneModelService());
		this.modelService = instantiationService.get(IStandaloneModelService);
		this.languageFeaturesService = instantiationService.get(IStandaloneLanguageFeaturesService);
		this.syntaxWorkerFactory = overrides.syntaxWorkerFactory ?? createSyntaxWorkerFactory();
		this.completionWorkerFactory = overrides.completionWorkerFactory ?? createCompletionWorkerFactory();
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
