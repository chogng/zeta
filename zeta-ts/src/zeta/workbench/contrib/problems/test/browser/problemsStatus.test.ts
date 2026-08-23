import assert from "node:assert/strict";
import test from "node:test";
import { Emitter } from "../../../../../base/common/event.js";
import { LanguageDiagnosticSeverity } from "../../../../../editor/common/languages/languageResults.js";
import type { LanguageDiagnosticSnapshot } from "../../../../../editor/common/services/languageDiagnosticsService.js";
import { ProblemsStatusContribution } from "../../../../../workbench/contrib/problems/browser/problemsStatus.js";
import type { ILanguageDiagnosticsService } from "../../../../../workbench/services/language/common/languageDiagnosticsService.js";
import { StatusbarAlignment, StatusbarService } from "../../../../../workbench/services/statusbar/browser/statusbar.js";
import type { IViewsService } from "../../../../../workbench/services/views/browser/viewsService.js";

test("Problems status projects and updates workspace error and warning counts", () => {
	const changes = new Emitter<never>();
	let diagnostics = snapshots(LanguageDiagnosticSeverity.Error, LanguageDiagnosticSeverity.Error, LanguageDiagnosticSeverity.Warning, LanguageDiagnosticSeverity.Information);
	const diagnosticsService = {
		onDidChangeDiagnostics: changes.event,
		getAllDiagnostics: () => diagnostics,
	} as unknown as ILanguageDiagnosticsService;
	const focusedViews: string[] = [];
	const viewsService = {
		focusView: (viewId: string) => { focusedViews.push(viewId); return true; },
	} as unknown as IViewsService;
	using statusbar = new StatusbarService();
	using contribution = new ProblemsStatusContribution({ statusbarService: statusbar, diagnosticsService, viewsService });

	assert.deepEqual(statusbar.getEntries(StatusbarAlignment.Left).map(item => item.id), ["zeta.status.problems"]);
	assert.deepEqual(statusbar.getEntries(StatusbarAlignment.Left)[0]?.entry.segments?.map(segment => segment.text), ["2", "1"]);
	assert.equal(statusbar.getEntries(StatusbarAlignment.Left)[0]?.entry.run?.(), true);
	assert.deepEqual(focusedViews, ["zeta.problems"]);

	diagnostics = snapshots(LanguageDiagnosticSeverity.Warning, LanguageDiagnosticSeverity.Warning);
	changes.fire(undefined as never);
	assert.deepEqual(statusbar.getEntries(StatusbarAlignment.Left)[0]?.entry.segments?.map(segment => segment.text), ["0", "2"]);
	assert.equal(statusbar.getEntries(StatusbarAlignment.Left)[0]?.entry.tooltip, "Warnings: 2");

	contribution.dispose();
	assert.deepEqual(statusbar.getEntries(StatusbarAlignment.Left), []);
});

function snapshots(...severities: readonly LanguageDiagnosticSeverity[]): readonly LanguageDiagnosticSnapshot[] {
	return [{ diagnostics: severities.map(severity => ({ severity })) }] as unknown as readonly LanguageDiagnosticSnapshot[];
}
