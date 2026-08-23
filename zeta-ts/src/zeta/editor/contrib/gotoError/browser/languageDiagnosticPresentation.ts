import { type TextDecorationCollection } from "../../../common/model/decorationCollection.js";
import { LanguageDiagnosticSeverity, type LanguageDiagnostic } from "../../../common/languages/languageResults.js";
import { DecorationPresentation, createAsterDecorationSource, type DecorationSource } from "../../../browser/viewparts/decorations/decorationPresentation.js";

/**
 * Creates Aster's underline projection for caller-owned language diagnostics.
 *
 * Every normalized severity maps to one component-owned underline presentation.
 */
export function createAsterLanguageDiagnosticSource(collection: TextDecorationCollection<LanguageDiagnostic>): DecorationSource {
	return createAsterDecorationSource(
		collection,
		decoration => resolveAsterLanguageDiagnosticPresentation(
			decoration.metadata.severity,
		),
		decoration => diagnosticHoverText(decoration.metadata),
	);
}

function diagnosticHoverText(diagnostic: LanguageDiagnostic): string {
	const prefix = [diagnostic.source, diagnostic.code].filter(value => value !== undefined).join(" ");
	return prefix.length === 0 ? diagnostic.message : `${prefix}: ${diagnostic.message}`;
}

export function resolveAsterLanguageDiagnosticPresentation(severity: LanguageDiagnosticSeverity): DecorationPresentation | undefined {
	switch (severity) {
		case LanguageDiagnosticSeverity.Error:
			return DecorationPresentation.ErrorUnderline;
		case LanguageDiagnosticSeverity.Warning:
			return DecorationPresentation.WarningUnderline;
		case LanguageDiagnosticSeverity.Information:
			return DecorationPresentation.InformationUnderline;
		case LanguageDiagnosticSeverity.Hint:
			return DecorationPresentation.HintUnderline;
		default:
			throw new TypeError(`Unknown language diagnostic severity '${severity}'`);
	}
}
