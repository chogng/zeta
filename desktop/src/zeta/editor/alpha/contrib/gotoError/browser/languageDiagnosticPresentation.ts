import { type TextDecorationCollection } from "../../../common/model/decorationCollection.js";
import { LanguageDiagnosticSeverity, type LanguageDiagnostic } from "../../../common/languages/languageResults.js";
import { AlphaDecorationPresentation, createAlphaDecorationSource, type AlphaDecorationSource } from "../../../browser/view/decorationPresentation.js";

/**
 * Creates Alpha's underline projection for caller-owned language diagnostics.
 *
 * Every normalized severity maps to one component-owned underline presentation.
 */
export function createAlphaLanguageDiagnosticSource(collection: TextDecorationCollection<LanguageDiagnostic>): AlphaDecorationSource {
  return createAlphaDecorationSource(
    collection,
    decoration => resolveAlphaLanguageDiagnosticPresentation(
      decoration.metadata.severity,
    ),
    decoration => diagnosticHoverText(decoration.metadata),
  );
}

function diagnosticHoverText(diagnostic: LanguageDiagnostic): string {
  const prefix = [diagnostic.source, diagnostic.code].filter(value => value !== undefined).join(" ");
  return prefix.length === 0 ? diagnostic.message : `${prefix}: ${diagnostic.message}`;
}

export function resolveAlphaLanguageDiagnosticPresentation(severity: LanguageDiagnosticSeverity): AlphaDecorationPresentation | undefined {
  switch (severity) {
    case LanguageDiagnosticSeverity.Error:
      return AlphaDecorationPresentation.ErrorUnderline;
    case LanguageDiagnosticSeverity.Warning:
      return AlphaDecorationPresentation.WarningUnderline;
    case LanguageDiagnosticSeverity.Information:
      return AlphaDecorationPresentation.InformationUnderline;
    case LanguageDiagnosticSeverity.Hint:
      return AlphaDecorationPresentation.HintUnderline;
    default:
      throw new TypeError(`Unknown language diagnostic severity '${severity}'`);
  }
}
