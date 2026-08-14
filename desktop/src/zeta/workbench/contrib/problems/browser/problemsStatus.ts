import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { lxiconsLibrary } from "../../../../base/common/lxiconsLibrary.js";
import { LanguageDiagnosticSeverity } from "../../../../editor/common/languages/languageResults.js";
import type { IWorkbenchContribution } from "../../../common/contributions.js";
import type { ILanguageDiagnosticsService } from "../../../services/language/common/languageDiagnosticsService.js";
import { StatusbarAlignment, type IStatusbarEntry, type IStatusbarEntryAccessor, type IStatusbarService } from "../../../services/statusbar/browser/statusbar.js";

const ErrorPriority = 700;
const WarningPriority = 690;

export interface ProblemsStatusContributionOptions {
  readonly statusbarService: IStatusbarService;
  readonly diagnosticsService: ILanguageDiagnosticsService;
}

/** Projects workspace error and warning counts into the status bar. */
export class ProblemsStatusContribution extends DisposableOwner implements IWorkbenchContribution {
  private readonly errors: IStatusbarEntryAccessor;
  private readonly warnings: IStatusbarEntryAccessor;

  constructor(private readonly options: ProblemsStatusContributionOptions) {
    super();
    this.errors = this.own(options.statusbarService.addEntry(this.entry(LanguageDiagnosticSeverity.Error), {
      id: "zeta.status.problems.errors",
      alignment: StatusbarAlignment.Left,
      priority: ErrorPriority,
    }));
    this.warnings = this.own(options.statusbarService.addEntry(this.entry(LanguageDiagnosticSeverity.Warning), {
      id: "zeta.status.problems.warnings",
      alignment: StatusbarAlignment.Left,
      priority: WarningPriority,
    }));
    this.own(options.diagnosticsService.onDidChangeDiagnostics(() => this.update()));
  }

  private update(): void {
    this.errors.update(this.entry(LanguageDiagnosticSeverity.Error));
    this.warnings.update(this.entry(LanguageDiagnosticSeverity.Warning));
  }

  private entry(severity: LanguageDiagnosticSeverity.Error | LanguageDiagnosticSeverity.Warning): IStatusbarEntry {
    const count = this.count(severity);
    const label = severity === LanguageDiagnosticSeverity.Error ? "error" : "warning";
    return {
      icon: severity === LanguageDiagnosticSeverity.Error ? lxiconsLibrary.error : lxiconsLibrary.warning,
      text: String(count),
      ariaLabel: `${count} ${count === 1 ? label : `${label}s`}`,
      tooltip: `${count} workspace ${count === 1 ? label : `${label}s`}`,
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
