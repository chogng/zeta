import { type TextDecorationCollection } from '../../common/model/decorationCollection.js';
import { type LanguageDiagnostic } from '../../common/languages/languageResults.js';
import { createStanzaLanguageDiagnosticSource } from '../../contrib/gotoError/browser/languageDiagnosticPresentation.js';

/** Owns the browser decoration source derived from one diagnostic collection. */
export class MarkerDecorationsContribution {
	public readonly decorationSource;

	constructor(collection: TextDecorationCollection<LanguageDiagnostic>) {
		this.decorationSource = createStanzaLanguageDiagnosticSource(collection);
	}
}
