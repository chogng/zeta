import { Disposable, type IDisposable } from '../../../base/common/lifecycle.js';
import { createServiceIdentifier } from '../../../platform/instantiation/common/instantiation.js';
import { LanguageConfigurationRegistry, type LanguageConfiguration, type LanguageConfigurationChangeEvent, type LanguageConfigurationContributionInput, type LanguageConfigurationRegistration, type LanguageConfigurationRegistrationOptions, type LanguageConfigurationSource, type ResolvedLanguageConfiguration } from '../languages/languageConfiguration.js';
import { type Event } from '../../../base/common/event.js';

export interface ILanguageConfigurationService extends LanguageConfigurationSource, IDisposable {
	readonly configurations: LanguageConfigurationRegistry;
	readonly onDidChangeConfiguration: Event<LanguageConfigurationChangeEvent>;
	register(languageId: string, configuration: LanguageConfiguration, options?: LanguageConfigurationRegistrationOptions): IDisposable;
	registerMany(contributions: readonly LanguageConfigurationContributionInput[]): LanguageConfigurationRegistration;
}

export const ILanguageConfigurationService = createServiceIdentifier<ILanguageConfigurationService>('languageConfigurationService');

/** Owns composable editing rules independently of language identities and providers. */
export class LanguageConfigurationService extends Disposable implements ILanguageConfigurationService {
	public readonly configurations = this._register(new LanguageConfigurationRegistry());
	public readonly onDidChangeConfiguration = this.configurations.onDidChangeConfiguration;

	public register(languageId: string, configuration: LanguageConfiguration, options: LanguageConfigurationRegistrationOptions = {}): IDisposable {
		return this.configurations.register(languageId, configuration, options);
	}

	public registerMany(contributions: readonly LanguageConfigurationContributionInput[]): LanguageConfigurationRegistration {
		return this.configurations.registerMany(contributions);
	}

	public getLanguageConfiguration(languageId: string): ResolvedLanguageConfiguration {
		return this.configurations.getLanguageConfiguration(languageId);
	}
}
