import assert from "node:assert/strict";
import test from "node:test";
import { Emitter } from "../../../../../base/common/event.js";
import { URI } from "../../../../../base/common/uri.js";
import { TextPosition, TextRange } from "../../../../../editor/common/core/text.js";
import { LanguageDiagnosticSeverity } from "../../../../../editor/common/languages/languageResults.js";
import { MarkerService, MarkerSeverity } from "../../../../../platform/markers/common/markers.js";
import { LanguageDiagnosticsMarkerBridge } from "../../browser/languageDiagnosticsMarkerBridge.js";
import type { ILanguageDiagnosticsService, LanguageDiagnosticSnapshot } from "../../common/languageDiagnosticsService.js";

test("language diagnostics bridge projects current diagnostics into markers", () => {
	const resource = URI.file("C:\\project\\src\\main.rs");
	const changes = new Emitter<URI>();
	let snapshots: readonly LanguageDiagnosticSnapshot[] = [snapshot(resource, LanguageDiagnosticSeverity.Error, "broken")];
	const diagnostics = {
		onDidChangeDiagnostics: changes.event,
		getAllDiagnostics: () => snapshots,
	} as unknown as ILanguageDiagnosticsService;
	using markerService = new MarkerService();
	using bridge = new LanguageDiagnosticsMarkerBridge(diagnostics, markerService);

	assert.deepEqual(markerService.getAll().map(marker => ({
		severity: marker.severity,
		message: marker.message,
		source: marker.source,
	})), [{ severity: MarkerSeverity.Error, message: "broken", source: "rust-analyzer" }]);

	snapshots = [snapshot(resource, LanguageDiagnosticSeverity.Warning, "unused")];
	changes.fire(resource);
	assert.deepEqual(markerService.getAll().map(marker => ({ severity: marker.severity, message: marker.message })), [{ severity: MarkerSeverity.Warning, message: "unused" }]);

	bridge.dispose();
	snapshots = [];
	changes.fire(resource);
	assert.equal(markerService.getAll().length, 0);
});

function snapshot(resource: URI, severity: LanguageDiagnosticSeverity, message: string): LanguageDiagnosticSnapshot {
	return {
		resource,
		revision: 1,
		diagnostics: [{
			range: TextRange.from(TextPosition.at(1, 2), TextPosition.at(1, 5)),
			severity,
			message,
			source: "rust-analyzer",
		}],
	};
}
