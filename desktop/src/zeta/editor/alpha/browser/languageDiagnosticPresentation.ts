import { type TextDecorationCollection } from "../common/decoration.js";
import { LanguageDiagnosticSeverity, type LanguageDiagnostic } from "../common/languageResults.js";
import { AlphaDecorationPresentation, createAlphaDecorationSource, type AlphaDecorationSource } from "./decorationPresentation.js";

/**
 * Creates Alpha's underline projection for caller-owned language diagnostics.
 *
 * Information and Hint remain in common state but have no underline until the
 * component owns dedicated presentations and semantic theme tokens.
 */
export function createAlphaLanguageDiagnosticSource(collection: TextDecorationCollection<LanguageDiagnostic>): AlphaDecorationSource {
  return createAlphaDecorationSource(
    collection,
    decoration => resolveAlphaLanguageDiagnosticPresentation(
      decoration.metadata.severity,
    ),
  );
}

export function resolveAlphaLanguageDiagnosticPresentation(severity: LanguageDiagnosticSeverity): AlphaDecorationPresentation | undefined {
  switch (severity) {
    case LanguageDiagnosticSeverity.Error:
      return AlphaDecorationPresentation.ErrorUnderline;
    case LanguageDiagnosticSeverity.Warning:
      return AlphaDecorationPresentation.WarningUnderline;
    case LanguageDiagnosticSeverity.Information:
    case LanguageDiagnosticSeverity.Hint:
      return undefined;
    default:
      throw new TypeError(`Unknown language diagnostic severity '${severity}'`);
  }
}
