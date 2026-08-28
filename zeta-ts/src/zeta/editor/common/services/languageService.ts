import { Disposable, type IDisposable } from '../../../base/common/lifecycle.js';
import type { TextResourceLanguageInput } from '../../../platform/language/common/textResourceLanguage.js';
import { createServiceIdentifier } from '../../../platform/instantiation/common/instantiation.js';
import { LanguageRegistry, type LanguageDescription, type LanguageDescriptionContribution, type LanguageDescriptionRegistration, type LanguageRegistrationOptions } from '../languages/languageRegistry.js';

export interface ILanguageService extends IDisposable {
	readonly languages: LanguageRegistry;
	registerLanguage(description: LanguageDescription, options?: LanguageRegistrationOptions): IDisposable;
	registerLanguages(contributions: readonly LanguageDescriptionContribution[]): LanguageDescriptionRegistration;
	resolveLanguageId(input: TextResourceLanguageInput): string | undefined;
}

export const ILanguageService = createServiceIdentifier<ILanguageService>('languageService');

/** Owns language identities and file associations independently of feature providers. */
export class LanguageService extends Disposable implements ILanguageService {
	public readonly languages = this._register(new LanguageRegistry());

	public registerLanguage(description: LanguageDescription, options: LanguageRegistrationOptions = {}): IDisposable {
		return this.languages.register(description, options);
	}

	public registerLanguages(contributions: readonly LanguageDescriptionContribution[]): LanguageDescriptionRegistration {
		return this.languages.registerMany(contributions);
	}

	public resolveLanguageId(input: TextResourceLanguageInput): string | undefined {
		return this.languages.resolveLanguageId(input);
	}
}
