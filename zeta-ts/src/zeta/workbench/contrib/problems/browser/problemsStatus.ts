import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { lxiconsLibrary } from "../../../../base/common/lxiconsLibrary.js";
import { LanguageDiagnosticSeverity } from "../../../../editor/common/languages/languageResults.js";
import type { IWorkbenchContribution } from "../../../common/contributions.js";
import type { ILanguageDiagnosticsService } from "../../../services/language/common/languageDiagnosticsService.js";
import { StatusbarAlignment, type IStatusbarEntry, type IStatusbarEntryAccessor, type IStatusbarService } from "../../../services/statusbar/browser/statusbar.js";
import type { IViewsService } from "../../../services/views/browser/viewsService.js";

const ProblemsPriority = 700;

export interface ProblemsStatusContributionOptions {
	readonly statusbarService: IStatusbarService;
	readonly diagnosticsService: ILanguageDiagnosticsService;
	readonly viewsService: IViewsService;
}

/** Projects workspace error and warning counts into the status bar. */
export class ProblemsStatusContribution extends DisposableOwner implements IWorkbenchContribution {
	private readonly status: IStatusbarEntryAccessor;

	constructor(private readonly options: ProblemsStatusContributionOptions) {
		super();
		this.status = this.own(options.statusbarService.addEntry(this.entry(), {
			id: "zeta.status.problems",
			alignment: StatusbarAlignment.Left,
			priority: ProblemsPriority,
		}));
		this.own(options.diagnosticsService.onDidChangeDiagnostics(() => this.update()));
	}

	private update(): void {
		this.status.update(this.entry());
	}

	private entry(): IStatusbarEntry {
		const errors = this.count(LanguageDiagnosticSeverity.Error);
		const warnings = this.count(LanguageDiagnosticSeverity.Warning);
		const summaries: string[] = [];
		if (errors > 0) summaries.push(`Errors: ${errors}`);
		if (warnings > 0) summaries.push(`Warnings: ${warnings}`);
		const tooltip = summaries.length === 0 ? "No Problems" : summaries.join(", ");
		return {
			text: "",
			segments: [
				{ icon: lxiconsLibrary.error, text: packNumber(errors) },
				{ icon: lxiconsLibrary.warning, text: packNumber(warnings) },
			],
			ariaLabel: tooltip,
			tooltip,
			run: () => this.options.viewsService.focusView("zeta.problems"),
		};
	}

	private count(severity: LanguageDiagnosticSeverity): number {
		let count = 0;
		for (const snapshot of this.options.diagnosticsService.getAllDiagnostics()) {
			for (const diagnostic of snapshot.diagnostics) {
				if (diagnostic.severity === severity) count += 1;
			}
		}
		return count;
	}
}

function packNumber(count: number): string {
	if (count > 9_999) return "10K+";
	if (count > 999) return `${String(count)[0]}K`;
	return String(count);
}
