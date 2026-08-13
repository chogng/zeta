import { addDisposableListener } from "../../../../base/browser/dom.js";
import { type ITestProfile, type ITestRun, type ITestingService } from "../../../services/testing/common/testingService.js";
import { type ITerminalService } from "../../../services/terminal/common/terminal.js";
import { type IViewsService } from "../../../services/views/browser/viewsService.js";
import { ViewPane, type IViewPaneOptions } from "../../../browser/parts/views/viewPane.js";
import { TERMINAL_VIEW_ID } from "../../terminal/browser/terminal.contribution.js";

/** Test profiles and their latest task-backed run status. */
export class TestingViewPane extends ViewPane {
  private readonly statusElement: HTMLDivElement;
  private readonly profilesElement: HTMLUListElement;
  private renderedProfiles: readonly ITestProfile[] = [];
  private renderedRuns: readonly ITestRun[] = [];
  private error: string | undefined;
  private refreshing = false;

  constructor(options: IViewPaneOptions, private readonly testingService: ITestingService, private readonly terminalService: ITerminalService, private readonly viewsService: IViewsService) {
    super(options);
    this.contentElement.classList.add("zeta-testing");
    const controls = options.ownerDocument.createElement("div");
    controls.className = "zeta-testing-controls";
    const runAll = button(options.ownerDocument, "Run All", "zeta-testing-run-all");
    const refresh = button(options.ownerDocument, "Refresh", "zeta-testing-refresh");
    controls.append(runAll, refresh);
    this.statusElement = options.ownerDocument.createElement("div");
    this.statusElement.className = "zeta-testing-status";
    this.statusElement.setAttribute("role", "status");
    this.profilesElement = options.ownerDocument.createElement("ul");
    this.profilesElement.className = "zeta-testing-list";
    this.profilesElement.setAttribute("aria-label", "Test profiles");
    this.contentElement.append(controls, this.statusElement, this.profilesElement);
    this.own(addDisposableListener(runAll, "click", () => this.runAll()));
    this.own(addDisposableListener(refresh, "click", () => this.refresh()));
    this.own(addDisposableListener(this.profilesElement, "click", event => this.activate(event)));
    this.own(testingService.onDidChangeProfiles(() => this.render()));
    this.own(testingService.onDidStartRun(() => this.render()));
    this.own(testingService.onDidChangeRun(() => this.render()));
    this.render();
    this.refresh();
  }

  private refresh(): void {
    if (this.refreshing) return;
    this.refreshing = true;
    this.error = undefined;
    this.render();
    void this.testingService.refresh().catch(error => { this.error = message(error, "Could not discover tests."); }).finally(() => { this.refreshing = false; this.render(); });
  }

  private runAll(): void {
    this.error = undefined;
    void this.testingService.runAll().catch(error => { this.error = message(error, "Could not run tests."); this.render(); });
  }

  private activate(event: Event): void {
    const target = event.target;
    if (!(target instanceof this.element.ownerDocument.defaultView!.Element)) return;
    const profileIndex = Number(target.closest<HTMLButtonElement>(".zeta-testing-run")?.dataset.profileIndex);
    if (Number.isSafeInteger(profileIndex) && this.renderedProfiles[profileIndex]) {
      void this.testingService.run(this.renderedProfiles[profileIndex]!).catch(error => { this.error = message(error, "Could not run test profile."); this.render(); });
      return;
    }
    const runIndex = Number(target.closest<HTMLButtonElement>(".zeta-testing-run-result")?.dataset.runIndex);
    const run = Number.isSafeInteger(runIndex) ? this.renderedRuns[runIndex] : undefined;
    if (run) {
      this.terminalService.setActiveInstance(run.taskRun.terminal);
      this.viewsService.focusView(TERMINAL_VIEW_ID);
    }
  }

  private render(): void {
    this.renderedProfiles = this.testingService.profiles;
    this.renderedRuns = this.testingService.runs.slice(-20).reverse();
    this.profilesElement.replaceChildren(...this.renderedProfiles.map((profile, index) => renderProfile(this.element.ownerDocument, profile, index)), ...this.renderedRuns.map((run, index) => renderRun(this.element.ownerDocument, run, index)));
    const completed = this.renderedRuns.filter(run => run.status !== "running");
    const passed = completed.filter(run => run.status === "passed").length;
    this.statusElement.textContent = this.error ?? (this.refreshing ? "Discovering tests…" : this.renderedProfiles.length === 0 ? "No test tasks found. Mark a task as the test group or add a test script." : completed.length === 0 ? `${this.renderedProfiles.length} test ${this.renderedProfiles.length === 1 ? "profile" : "profiles"}.` : `${passed} passed, ${completed.length - passed} not passed.`);
  }
}

function renderProfile(document: Document, profile: ITestProfile, index: number): HTMLLIElement {
  const item = document.createElement("li");
  item.className = "zeta-testing-profile";
  const run = button(document, "Run", "zeta-testing-run");
  run.dataset.profileIndex = String(index);
  const label = document.createElement("span");
  label.className = "zeta-testing-label";
  label.textContent = profile.label;
  const detail = document.createElement("span");
  detail.className = "zeta-testing-detail";
  detail.textContent = profile.detail ?? profile.source;
  item.append(run, label, detail);
  return item;
}

function renderRun(document: Document, run: ITestRun, index: number): HTMLLIElement {
  const item = document.createElement("li");
  item.className = `zeta-testing-result ${run.status}`;
  const open = button(document, `${run.profile.label} — ${run.status}`, "zeta-testing-run-result");
  open.dataset.runIndex = String(index);
  item.append(open);
  return item;
}

function button(document: Document, label: string, className: string): HTMLButtonElement {
  const element = document.createElement("button");
  element.type = "button";
  element.className = className;
  element.textContent = label;
  return element;
}

function message(error: unknown, fallback: string): string {
  return error instanceof Error ? error.message : fallback;
}
