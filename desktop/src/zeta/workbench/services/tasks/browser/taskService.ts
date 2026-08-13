import { Emitter, type Event } from "../../../../base/common/event.js";
import { DisposableOwner, type IDisposable } from "../../../../base/common/lifecycle.js";
import { type URI } from "../../../../base/common/uri.js";
import { FileKind, FileNotFoundError, type IFileService } from "../../../../platform/files/common/files.js";
import { type IWorkspaceContextService } from "../../../../platform/workspace/common/workspace.js";
import { type ITerminalCommandStatusEvent, type ITerminalInstance, type ITerminalService } from "../../terminal/common/terminal.js";
import { type ITaskRun, type ITaskService, type IWorkspaceTask, type TaskRunStatus } from "../common/taskService.js";
import { cargoWorkspaceTasks, parsePackageTasks, parseWorkspaceTasks } from "../common/workspaceTasks.js";

const TASK_TERMINAL_DIMENSIONS = Object.freeze({ rows: 24, cols: 80 });

/** Workspace task discovery and integrated-Terminal execution. */
export class TaskService extends DisposableOwner implements ITaskService {
  private readonly changeTasksEmitter = this.own(new Emitter<readonly IWorkspaceTask[]>());
  private readonly startTaskEmitter = this.own(new Emitter<ITaskRun>());
  private readonly changeTaskRunEmitter = this.own(new Emitter<ITaskRun>());
  private readonly runs = new Set<TaskRun>();
  private currentTasks: readonly IWorkspaceTask[] = Object.freeze([]);
  private refreshGeneration = 0;
  private loaded = false;
  private _lastRun: TaskRun | undefined;

  readonly onDidChangeTasks: Event<readonly IWorkspaceTask[]> = this.changeTasksEmitter.event;
  readonly onDidStartTask: Event<ITaskRun> = this.startTaskEmitter.event;
  readonly onDidChangeTaskRun: Event<ITaskRun> = this.changeTaskRunEmitter.event;

  constructor(private readonly fileService: IFileService, private readonly workspace: IWorkspaceContextService, private readonly terminalService: ITerminalService) {
    super();
    this.own(fileService.onDidChangeFiles(event => {
      if (this.loaded && affectsTaskConfiguration(event.resources)) void this.refresh().catch(reportTaskError);
    }));
    this.own(workspace.onDidChangeWorkspace(() => {
      this.refreshGeneration += 1;
      this.loaded = false;
      this.setTasks(Object.freeze([]));
    }));
    this.defer(() => {
      for (const run of this.runs) run.dispose();
      this.runs.clear();
    });
  }

  get tasks(): readonly IWorkspaceTask[] { return this.currentTasks; }
  get activeRuns(): readonly ITaskRun[] { return [...this.runs].filter(run => run.status === "running"); }
  get lastRun(): ITaskRun | undefined { return this._lastRun; }

  async refresh(): Promise<readonly IWorkspaceTask[]> {
    const generation = ++this.refreshGeneration;
    const root = this.workspace.getWorkspace().folders[0]?.uri;
    if (!root) {
      this.loaded = true;
      this.setTasks(Object.freeze([]));
      return this.currentTasks;
    }
    const discovered: IWorkspaceTask[] = [];
    const tasksJson = await this.readOptional(childResource(root, ".vscode/tasks.json"));
    if (tasksJson !== undefined) discovered.push(...parseWorkspaceTasks(tasksJson));
    const packageJson = await this.readOptional(childResource(root, "package.json"));
    if (packageJson !== undefined) discovered.push(...parsePackageTasks(packageJson, await this.packageManager(root)));
    if (await this.exists(childResource(root, "Cargo.toml"))) discovered.push(...cargoWorkspaceTasks());
    if (generation !== this.refreshGeneration) return this.currentTasks;
    this.loaded = true;
    this.setTasks(deduplicateTasks(discovered));
    return this.currentTasks;
  }

  async run(task: IWorkspaceTask): Promise<ITaskRun> {
    assertKnownTask(task, this.currentTasks);
    const terminal = await this.terminalService.createTerminal({ dimensions: TASK_TERMINAL_DIMENSIONS, profile: { type: "default" }, title: `Task: ${task.label}` });
    const run = this.own(new TaskRun(task, terminal, current => {
      this.changeTaskRunEmitter.fire(current);
      if (current.status !== "running") this.runs.delete(current);
    }));
    this.runs.add(run);
    this._lastRun = run;
    this.startTaskEmitter.fire(run);
    const command = substituteWorkspaceVariables(task.command, this.workspace.getWorkspace().folders[0]?.uri);
    terminal.write(`${taskTerminalCommand(command, terminal.profile.profileId)}\r`);
    return run;
  }

  async terminate(run: ITaskRun): Promise<void> {
    if (!this.runs.has(run as TaskRun)) return;
    (run as TaskRun).cancel();
    await this.terminalService.closeTerminal(run.terminal);
  }

  private setTasks(tasks: readonly IWorkspaceTask[]): void {
    if (taskListsEqual(this.currentTasks, tasks)) return;
    this.currentTasks = tasks;
    this.changeTasksEmitter.fire(tasks);
  }

  private async packageManager(root: URI): Promise<"npm" | "pnpm" | "yarn"> {
    if (await this.exists(childResource(root, "pnpm-lock.yaml"))) return "pnpm";
    if (await this.exists(childResource(root, "yarn.lock"))) return "yarn";
    return "npm";
  }

  private async exists(resource: URI): Promise<boolean> {
    try { return (await this.fileService.stat(resource)).kind === FileKind.File; }
    catch (error) { if (error instanceof FileNotFoundError) return false; throw error; }
  }

  private async readOptional(resource: URI): Promise<string | undefined> {
    if (!await this.exists(resource)) return undefined;
    return (await this.fileService.readFile(resource)).content;
  }
}

class TaskRun extends DisposableOwner implements ITaskRun {
  private readonly changeStatusEmitter = this.own(new Emitter<TaskRunStatus>());
  private commandId: string | undefined;
  private _status: TaskRunStatus = "running";
  private _exitCode: number | undefined;
  readonly onDidChangeStatus: Event<TaskRunStatus> = this.changeStatusEmitter.event;

  constructor(readonly task: IWorkspaceTask, readonly terminal: ITerminalInstance, private readonly onChange: (run: TaskRun) => void) {
    super();
    this.own(terminal.onDidChangeCommandStatus(event => this.acceptCommandStatus(event)));
    this.own(terminal.onDidExit(exitCode => {
      if (this._status === "running") this.setStatus(exitCode === 0 ? "succeeded" : "failed", exitCode);
    }));
    this.own(terminal.onDidChangeState(state => {
      if (this._status === "running" && (state === "disconnected" || state === "error")) this.setStatus(state === "error" ? "failed" : "canceled", undefined);
    }));
  }

  get status(): TaskRunStatus { return this._status; }
  get exitCode(): number | undefined { return this._exitCode; }

  cancel(): void {
    this.setStatus("canceled", undefined);
  }

  private acceptCommandStatus(event: ITerminalCommandStatusEvent): void {
    if (!this.commandId && event.status === "running") this.commandId = event.commandId;
    if (event.commandId !== this.commandId || event.status === "running") return;
    this.setStatus(event.status, event.exitCode);
  }

  private setStatus(status: TaskRunStatus, exitCode: number | undefined): void {
    if (this._status !== "running") return;
    this._status = status;
    this._exitCode = exitCode;
    this.changeStatusEmitter.fire(status);
    this.onChange(this);
  }
}

function childResource(root: URI, relativePath: string): URI {
  const base = root.path.endsWith("/") ? root.path.slice(0, -1) : root.path;
  return root.withPath(`${base}/${relativePath.split("/").map(encodeURIComponent).join("/")}`);
}

function affectsTaskConfiguration(resources: readonly URI[] | undefined): boolean {
  return resources === undefined || resources.some(resource => /\/(?:tasks\.json|package\.json|pnpm-lock\.yaml|yarn\.lock|Cargo\.toml)$/i.test(resource.path));
}

function deduplicateTasks(tasks: readonly IWorkspaceTask[]): readonly IWorkspaceTask[] {
  const unique = new Map<string, IWorkspaceTask>();
  for (const task of tasks) if (!unique.has(task.id)) unique.set(task.id, task);
  return Object.freeze([...unique.values()].sort((left, right) => taskOrder(left.group) - taskOrder(right.group) || left.label.localeCompare(right.label)));
}

function taskOrder(group: IWorkspaceTask["group"]): number {
  return group === "build" ? 0 : group === "test" ? 1 : group === "run" ? 2 : 3;
}

function taskListsEqual(left: readonly IWorkspaceTask[], right: readonly IWorkspaceTask[]): boolean {
  return JSON.stringify(left) === JSON.stringify(right);
}

function assertKnownTask(task: IWorkspaceTask, tasks: readonly IWorkspaceTask[]): void {
  const current = tasks.find(candidate => candidate.id === task.id);
  if (!current || current.command !== task.command) throw new Error("Task is no longer present in the current workspace configuration");
}

function substituteWorkspaceVariables(command: string, root: URI | undefined): string {
  if (!root) return command;
  const workspaceFolder = root.scheme === "file" ? root.fsPath : decodeURIComponent(root.path);
  const basename = workspaceFolder.replace(/[\\/]+$/, "").split(/[\\/]/).at(-1) ?? "";
  return command.replaceAll("${workspaceFolder}", workspaceFolder).replaceAll("${workspaceFolderBasename}", basename);
}

function taskTerminalCommand(command: string, profileId: string): string {
  if (profileId === "fish") return `${command}; set __zeta_task_exit $status; exit $__zeta_task_exit`;
  if (profileId === "bash" || profileId === "zsh" || profileId === "sh" || profileId === "git-bash" || profileId === "default") return `${command}; __zeta_task_exit=$?; exit $__zeta_task_exit`;
  return command;
}

function reportTaskError(error: unknown): void {
  console.error("Could not refresh workspace tasks", error);
}
