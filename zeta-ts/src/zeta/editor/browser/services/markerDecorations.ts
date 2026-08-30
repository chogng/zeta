import { type TextEditorCapabilityContribution, type TextEditorContributionConfigurationContext } from '../editorExtensions.js';
import { TextEditorCapability } from '../../contrib/textEditorCapabilities.js';
import { createStanzaLanguageDiagnosticSource } from '../../contrib/gotoError/browser/languageDiagnosticPresentation.js';

/** Ensures diagnostic collections are projected as editor decorations. */
export const MarkerDecorationsContribution: TextEditorCapabilityContribution = Object.freeze({
	id: 'editor.contrib.markerDecorations',
	configure: (context: TextEditorContributionConfigurationContext) => context.addDecorationSource(createStanzaLanguageDiagnosticSource(context.getCapability(TextEditorCapability.diagnosticDecorations))),
});
