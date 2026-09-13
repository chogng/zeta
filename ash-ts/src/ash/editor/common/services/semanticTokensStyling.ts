import { createDecorator } from '../../../platform/instantiation/common/instantiation.js';
import { type LanguageSemanticTokensProvider } from '../languages.js';
import { type SemanticTokensProviderStyling } from './semanticTokensProviderStyling.js';

export const ISemanticTokensStylingService = createDecorator<ISemanticTokensStylingService>('semanticTokensStylingService');

export type DocumentTokensProvider = LanguageSemanticTokensProvider;

/** Owns provider-scoped token styling identities. */
export interface ISemanticTokensStylingService {
	readonly _serviceBrand: undefined;
	getStyling(provider: DocumentTokensProvider): SemanticTokensProviderStyling;
}
