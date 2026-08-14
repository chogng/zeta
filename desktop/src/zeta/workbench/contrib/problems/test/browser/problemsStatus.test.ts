import assert from "node:assert/strict";
import test from "node:test";
import { Emitter } from "../../../../../base/common/event.js";
import { LanguageDiagnosticSeverity } from "../../../../../editor/common/languages/languageResults.js";
import type { LanguageDiagnosticSnapshot } from "../../../../../editor/common/services/languageDiagnosticsService.js";
import { ProblemsStatusContribution } from "../../../../../workbench/contrib/problems/browser/problemsStatus.js";
import type { ILanguageDiagnosticsService } from "../../../../../workbench/services/language/common/languageDiagnosticsService.js";
import { StatusbarAlignment, StatusbarService } from "../../../../../workbench/services/statusbar/browser/statusbar.js";

test("Problems status projects and updates workspace error and warning counts", () => {
  const changes = new Emitter<never>();
  let diagnostics = snapshots(LanguageDiagnosticSeverity.Error, LanguageDiagnosticSeverity.Error, LanguageDiagnosticSeverity.Warning, LanguageDiagnosticSeverity.Information);
  const diagnosticsService = {
    onDidChangeDiagnostics: changes.event,
    getAllDiagnostics: () => diagnostics,
  } as unknown as ILanguageDiagnosticsService;
  using statusbar = new StatusbarService();
  using contribution = new ProblemsStatusContribution({ statusbarService: statusbar, diagnosticsService });

  assert.deepEqual(statusbar.getEntries(StatusbarAlignment.Left).map(item => item.id), [
    "zeta.status.problems.errors",
    "zeta.status.problems.warnings",
  ]);
  assert.deepEqual(statusbar.getEntries(StatusbarAlignment.Left).map(item => item.entry.text), ["2", "1"]);

  diagnostics = snapshots(LanguageDiagnosticSeverity.Warning, LanguageDiagnosticSeverity.Warning);
  changes.fire(undefined as never);
  assert.deepEqual(statusbar.getEntries(StatusbarAlignment.Left).map(item => item.entry.text), ["0", "2"]);

  contribution.dispose();
  assert.deepEqual(statusbar.getEntries(StatusbarAlignment.Left), []);
});

function snapshots(...severities: readonly LanguageDiagnosticSeverity[]): readonly LanguageDiagnosticSnapshot[] {
  return [{ diagnostics: severities.map(severity => ({ severity })) }] as unknown as readonly LanguageDiagnosticSnapshot[];
}
