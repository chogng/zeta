import { addDisposableListener } from "../../../../base/browser/dom.js";
import { type ITaskRun, type ITaskService, type IWorkspaceTask } from "../../../services/tasks/common/taskService.js";
import { ViewPane, type IViewPaneOptions } from "../../../browser/parts/views/viewPane.js";
import { TERMINAL_VIEW_ID } from "../../terminal/browser/terminal.contribution.js";
import { type IViewsService } from "../../../services/views/browser/viewsService.js";
import { type ITerminalService } from "../../../services/terminal/common/terminal.js";

/** Code-owned task catalog and execution status view. */
export class TasksViewPane extends ViewPane {
  private readonly statusElement: HTMLDivElement;
  private readonly listElement: HTMLUListElement;
  private renderedTasks: readonly IWorkspaceTask[] = [];
  private refreshing = false;
  private error: string | undefined;

  constructor(options: IViewPaneOptions, private readonly taskService: ITaskService, private readonly viewsService: IViewsService, private readonly terminalService: ITerminalService) {
    super(options);
    this.contentElement.classList.add("zeta-tasks");
    const controls = options.ownerDocument.createElement("div");
    controls.className = "zeta-tasks-controls";
    const refresh = options.ownerDocument.createElement("button");
    refresh.type = "button";
    refresh.className = "zeta-tasks-refresh";
    refresh.textContent = "Refresh";
    controls.append(refresh);
    this.statusElement = options.ownerDocument.createElement("div");
    this.statusElement.className = "zeta-tasks-status";
    this.statusElement.setAttribute("role", "status");
    this.listElement = options.ownerDocument.createElement("ul");
    this.listElement.className = "zeta-tasks-list";
    this.listElement.setAttribute("aria-label", "Workspace tasks");
    this.contentElement.append(controls, this.statusElement, this.listElement);
    this.own(addDisposableListener(refresh, "click", () => this.refresh()));
    this.own(addDisposableListener(this.listElement, "click", event => this.activate(event)));
    this.own(taskService.onDidChangeTasks(() => this.render()));
    this.own(taskService.onDidStartTask(() => this.render()));
    this.own(taskService.onDidChangeTaskRun(() => this.render()));
    this.render();
    this.refresh();
  }

  private refresh(): void {
    if (this.refreshing) return;
    this.refreshing = true;
    this.error = undefined;
    this.render();
    void this.taskService.refresh().catch(error => {
      this.error = error instanceof Error ? error.message : "Could not read workspace tasks.";
    }).finally(() => {
      this.refreshing = false;
      this.render();
    });
  }

  private activate(event: Event): void {
    const target = event.target;
    if (!(target instanceof this.element.ownerDocument.defaultView!.Element)) return;
    const taskIndex = Number(target.closest<HTMLButtonElement>(".zeta-tasks-run")?.dataset.taskIndex);
    if (Number.isSafeInteger(taskIndex) && this.renderedTasks[taskIndex]) {
      void this.taskService.run(this.renderedTasks[taskIndex]!).then(() => this.viewsService.focusView(TERMINAL_VIEW_ID)).catch(error => {
        this.error = error instanceof Error ? error.message : "Could not run task.";
        this.render();
      });
      return;
    }
    const terminalId = target.closest<HTMLButtonElement>(".zeta-task-run-button")?.dataset.terminalId;
    if (terminalId) {
      const terminal = this.terminalService.instances.find(candidate => candidate.id === terminalId);
      if (terminal) this.terminalService.setActiveInstance(terminal);
      this.viewsService.focusView(TERMINAL_VIEW_ID);
    }
  }

  private render(): void {
    this.renderedTasks = this.taskService.tasks;
    const runs = this.taskService.lastRun ? [this.taskService.lastRun] : [];
    this.listElement.replaceChildren(...this.renderedTasks.map((task, index) => this.renderTask(task, index)), ...runs.map(run => this.renderRun(run)));
    this.statusElement.textContent = this.error ?? (this.refreshing ? "Discovering workspace tasks…" : this.renderedTasks.length === 0 ? "No tasks found. Add .vscode/tasks.json, package.json scripts, or Cargo.toml." : `${this.renderedTasks.length} workspace ${this.renderedTasks.length === 1 ? "task" : "tasks"}.`);
  }

  private renderTask(task: IWorkspaceTask, index: number): HTMLLIElement {
    const item = this.element.ownerDocument.createElement("li");
    item.className = "zeta-task";
    const button = this.element.ownerDocument.createElement("button");
    button.type = "button";
    button.className = "zeta-tasks-run";
    button.dataset.taskIndex = String(index);
    const label = this.element.ownerDocument.createElement("span");
    label.className = "zeta-task-label";
    label.textContent = task.label;
    const source = this.element.ownerDocument.createElement("span");
    source.className = "zeta-task-source";
    source.textContent = `${task.group} · ${task.source}`;
    const command = this.element.ownerDocument.createElement("code");
    command.className = "zeta-task-command";
    command.textContent = task.command;
    button.append(label, source, command);
    item.append(button);
    return item;
  }

  private renderRun(run: ITaskRun): HTMLLIElement {
    const item = this.element.ownerDocument.createElement("li");
    item.className = `zeta-task-run ${run.status}`;
    const button = this.element.ownerDocument.createElement("button");
    button.type = "button";
    button.className = "zeta-task-run-button";
    button.dataset.terminalId = run.terminal.id;
    button.textContent = `Last: ${run.task.label} — ${run.status}${run.exitCode === undefined ? "" : ` (${run.exitCode})`}`;
    item.append(button);
    return item;
  }
}
