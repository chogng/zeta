import { addDisposableListener } from "../../../../base/browser/dom.js";
import { ActionBar } from "../../../../base/browser/ui/actionbar/actionbar.js";
import type { IAction } from "../../../../base/common/actions.js";
import { lxiconsLibrary } from "../../../../base/common/lxiconsLibrary.js";
import { type URI } from "../../../../base/common/uri.js";
import { LanguageDiagnosticSeverity, type LanguageDiagnostic } from "../../../../editor/common/languages/languageResults.js";
import { type IEditorService } from "../../../services/editor/common/editorService.js";
import { ViewPane, type IViewPaneOptions, type PartTitleProjection } from "../../../browser/parts/views/viewPane.js";
import { type ILanguageDiagnosticsService, type LanguageDiagnosticSnapshot } from "../../../services/language/common/languageDiagnosticsService.js";

interface ProblemEntry {
  readonly resource: URI;
  readonly diagnostic: LanguageDiagnostic;
}

const severities = Object.freeze([
  LanguageDiagnosticSeverity.Error,
  LanguageDiagnosticSeverity.Warning,
  LanguageDiagnosticSeverity.Information,
  LanguageDiagnosticSeverity.Hint,
]);

const severityLabels: Readonly<Record<LanguageDiagnosticSeverity, string>> = Object.freeze({
  [LanguageDiagnosticSeverity.Error]: "Errors",
  [LanguageDiagnosticSeverity.Warning]: "Warnings",
  [LanguageDiagnosticSeverity.Information]: "Information",
  [LanguageDiagnosticSeverity.Hint]: "Hints",
});

/** Aggregated workspace diagnostics with severity filtering and editor navigation. */
export class ProblemsViewPane extends ViewPane {
  private readonly filterInput: HTMLInputElement;
  private readonly titleActions: ActionBar;
  private readonly severityButtons = new Map<LanguageDiagnosticSeverity, HTMLButtonElement>();
  private readonly enabledSeverities = new Set<LanguageDiagnosticSeverity>(severities);
  private readonly statusElement: HTMLDivElement;
  private readonly resultsElement: HTMLUListElement;
  private renderedProblems: readonly ProblemEntry[] = [];
  private navigationError: string | undefined;
  private disposed = false;

  constructor(options: IViewPaneOptions, private readonly diagnosticsService: ILanguageDiagnosticsService, private readonly editorService: IEditorService) {
    super(options);
    this.contentElement.classList.add("zeta-problems");
    const document = options.ownerDocument;
    const controls = document.createElement("div");
    controls.className = "zeta-problems-controls";
    this.filterInput = document.createElement("input");
    this.filterInput.type = "text";
    this.filterInput.className = "zeta-problems-filter";
    this.filterInput.placeholder = "Filter problems";
    this.filterInput.setAttribute("aria-label", "Filter problems by message or file");
    this.filterInput.autocomplete = "off";
    this.filterInput.spellcheck = false;
    const focusFilterAction: IAction = {
      id: "zeta.problems.focusFilter",
      label: "Filter Problems",
      tooltip: "Filter Problems",
      icon: lxiconsLibrary.filter,
      enabled: true,
      checked: undefined,
      run: () => this.filterInput.focus(),
    };
    this.titleActions = this.own(new ActionBar({ ownerDocument: document, ariaLabel: "Problems actions", actions: [focusFilterAction] }));
    this.titleActions.element.classList.add("zeta-toolbar");
    const severityControls = document.createElement("div");
    severityControls.className = "zeta-problems-severities";
    severityControls.setAttribute("aria-label", "Problem severities");
    for (const severity of severities) {
      const button = document.createElement("button");
      button.type = "button";
      button.className = `zeta-problems-severity ${severity} checked`;
      button.textContent = severityLabels[severity];
      button.setAttribute("aria-pressed", "true");
      this.severityButtons.set(severity, button);
      severityControls.append(button);
      this.own(addDisposableListener(button, "click", () => this.toggleSeverity(severity)));
    }
    controls.append(this.filterInput, severityControls);
    this.statusElement = document.createElement("div");
    this.statusElement.className = "zeta-problems-status";
    this.statusElement.setAttribute("role", "status");
    this.statusElement.setAttribute("aria-live", "polite");
    this.resultsElement = document.createElement("ul");
    this.resultsElement.className = "zeta-problems-results";
    this.resultsElement.setAttribute("aria-label", "Problems");
    this.contentElement.append(controls, this.statusElement, this.resultsElement);
    this.own(addDisposableListener(this.filterInput, "input", () => this.render()));
    this.own(addDisposableListener(this.resultsElement, "click", event => {
      const button = event.target instanceof options.ownerDocument.defaultView!.Element ? event.target.closest<HTMLButtonElement>(".zeta-problems-item-button") : null;
      const index = Number(button?.dataset.problemIndex);
      const problem = Number.isSafeInteger(index) ? this.renderedProblems[index] : undefined;
      if (problem) void this.openProblem(problem);
    }));
    this.own(diagnosticsService.onDidChangeDiagnostics(() => this.render()));
    this.defer(() => { this.disposed = true; });
    this.render();
  }

  override get partTitleProjection(): PartTitleProjection {
    return { actions: this.titleActions.element };
  }

  private toggleSeverity(severity: LanguageDiagnosticSeverity): void {
    if (this.enabledSeverities.has(severity)) this.enabledSeverities.delete(severity);
    else this.enabledSeverities.add(severity);
    const button = this.severityButtons.get(severity)!;
    const enabled = this.enabledSeverities.has(severity);
    button.classList.toggle("checked", enabled);
    button.setAttribute("aria-pressed", String(enabled));
    this.render();
  }

  private render(): void {
    const all = problemEntries(this.diagnosticsService.getAllDiagnostics());
    const filter = this.filterInput.value.trim().toLocaleLowerCase();
    const visible = all.filter(entry => this.enabledSeverities.has(entry.diagnostic.severity) && matchesFilter(entry, filter));
    this.renderedProblems = visible;
    const indexes = new Map(visible.map((problem, index) => [problem, index]));
    this.resultsElement.replaceChildren(...groupProblems(visible).map(group => this.renderGroup(group.resource, group.problems, indexes)));
    this.statusElement.textContent = this.navigationError ?? statusMessage(all.length, visible.length, filter, this.enabledSeverities.size);
  }

  private renderGroup(resource: URI, problems: readonly ProblemEntry[], indexes: ReadonlyMap<ProblemEntry, number>): HTMLLIElement {
    const document = this.element.ownerDocument;
    const group = document.createElement("li");
    group.className = "zeta-problems-file";
    const heading = document.createElement("div");
    heading.className = "zeta-problems-file-heading";
    const name = document.createElement("span");
    name.className = "zeta-problems-file-name";
    name.textContent = resourceName(resource);
    const path = document.createElement("span");
    path.className = "zeta-problems-file-path";
    path.textContent = resourceParent(resource);
    const count = document.createElement("span");
    count.className = "zeta-problems-file-count";
    count.textContent = String(problems.length);
    heading.append(name, path, count);
    const rows = document.createElement("ul");
    rows.className = "zeta-problems-file-items";
    rows.append(...problems.map(problem => this.renderProblem(problem, indexes.get(problem)!)));
    group.append(heading, rows);
    return group;
  }

  private renderProblem(entry: ProblemEntry, index: number): HTMLLIElement {
    const document = this.element.ownerDocument;
    const item = document.createElement("li");
    item.className = `zeta-problems-item ${entry.diagnostic.severity}`;
    const button = document.createElement("button");
    button.type = "button";
    button.className = "zeta-problems-item-button";
    button.dataset.problemIndex = String(index);
    button.title = `${entry.diagnostic.message} — ${resourceName(entry.resource)}:${entry.diagnostic.range.start.lineIndex + 1}:${entry.diagnostic.range.start.columnIndex + 1}`;
    const marker = document.createElement("span");
    marker.className = "zeta-problems-marker";
    marker.setAttribute("aria-hidden", "true");
    const message = document.createElement("span");
    message.className = "zeta-problems-message";
    message.textContent = entry.diagnostic.message;
    const source = document.createElement("span");
    source.className = "zeta-problems-source";
    source.textContent = diagnosticSource(entry.diagnostic);
    const location = document.createElement("span");
    location.className = "zeta-problems-location";
    location.textContent = `[Ln ${entry.diagnostic.range.start.lineIndex + 1}, Col ${entry.diagnostic.range.start.columnIndex + 1}]`;
    button.append(marker, message, source, location);
    item.append(button);
    return item;
  }

  private async openProblem(entry: ProblemEntry): Promise<void> {
    this.navigationError = undefined;
    try {
      await this.editorService.openEditor({ resource: entry.resource, label: resourceName(entry.resource) }, { selection: entry.diagnostic.range });
      if (!this.disposed) this.editorService.focusActiveEditor();
    } catch (error) {
      if (this.disposed) return;
      this.navigationError = error instanceof Error ? error.message : "Could not open problem location.";
      this.render();
    }
  }
}

function problemEntries(snapshots: readonly LanguageDiagnosticSnapshot[]): readonly ProblemEntry[] {
  return snapshots.flatMap(snapshot => snapshot.diagnostics.map(diagnostic => ({ resource: snapshot.resource, diagnostic }))).sort(compareProblems);
}

function compareProblems(left: ProblemEntry, right: ProblemEntry): number {
  return left.resource.toString().localeCompare(right.resource.toString()) || severities.indexOf(left.diagnostic.severity) - severities.indexOf(right.diagnostic.severity) || left.diagnostic.range.start.compareTo(right.diagnostic.range.start) || left.diagnostic.message.localeCompare(right.diagnostic.message);
}

function groupProblems(problems: readonly ProblemEntry[]): readonly { readonly resource: URI; readonly problems: readonly ProblemEntry[] }[] {
  const groups = new Map<string, { readonly resource: URI; readonly problems: ProblemEntry[] }>();
  for (const problem of problems) {
    const key = problem.resource.toString();
    let group = groups.get(key);
    if (!group) {
      group = { resource: problem.resource, problems: [] };
      groups.set(key, group);
    }
    group.problems.push(problem);
  }
  return [...groups.values()];
}

function matchesFilter(entry: ProblemEntry, filter: string): boolean {
  if (!filter) return true;
  const diagnostic = entry.diagnostic;
  return `${diagnostic.message} ${diagnostic.source ?? ""} ${diagnostic.code ?? ""} ${resourceName(entry.resource)} ${resourceParent(entry.resource)}`.toLocaleLowerCase().includes(filter);
}

function statusMessage(total: number, visible: number, filter: string, enabledSeverityCount: number): string {
  if (total === 0) return "No problems have been detected in the workspace.";
  if (visible === 0) return filter || enabledSeverityCount < severities.length ? `No problems match the current filters (${total} total).` : "No problems have been detected in the workspace.";
  return visible === total ? `${total} ${total === 1 ? "problem" : "problems"} in the workspace.` : `${visible} of ${total} problems shown.`;
}

function diagnosticSource(diagnostic: LanguageDiagnostic): string {
  if (diagnostic.source && diagnostic.code !== undefined) return `${diagnostic.source}(${diagnostic.code})`;
  if (diagnostic.source) return diagnostic.source;
  if (diagnostic.code !== undefined) return String(diagnostic.code);
  return "";
}

function resourceName(resource: URI): string {
  const path = decodedPath(resource);
  return path.slice(path.lastIndexOf("/") + 1) || resource.authority || resource.toString();
}

function resourceParent(resource: URI): string {
  const path = decodedPath(resource);
  const separator = path.lastIndexOf("/");
  return separator > 0 ? path.slice(0, separator) : resource.authority;
}

function decodedPath(resource: URI): string {
  try { return decodeURIComponent(resource.path); }
  catch { return resource.path; }
}
