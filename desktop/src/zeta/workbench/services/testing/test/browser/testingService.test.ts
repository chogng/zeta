import assert from "node:assert/strict";
import test from "node:test";
import { Emitter, type Event } from "../../../../../base/common/event.js";
import { DisposableOwner, toDisposable } from "../../../../../base/common/lifecycle.js";
import { type ITaskRun, type ITaskService, type IWorkspaceTask, type TaskRunStatus } from "../../../../services/tasks/common/taskService.js";
import { type ITerminalInstance } from "../../../../services/terminal/common/terminal.js";
import { TestingService } from "../../browser/testingService.js";

test("TestingService exposes only test tasks and projects passed and failed runs", async () => {
  using tasks = new FakeTaskService([
    task("build", "Build", "build"),
    task("unit", "Unit", "test"),
    task("integration", "Integration", "test"),
  ]);
  using service = new TestingService(tasks);
  assert.deepEqual(service.profiles.map(profile => profile.label), ["Unit", "Integration"]);

  const run = await service.run(service.profiles[0]!);
  assert.equal(tasks.runs.length, 1);
  assert.equal(run.status, "running");
  tasks.runs[0]!.finish("succeeded");
  assert.equal(run.status, "passed");

  const rerun = await service.rerun(run);
  tasks.runs[1]!.finish("failed");
  assert.equal(rerun.status, "failed");
  assert.equal(service.runs.length, 2);

  const all = await service.runAll();
  assert.equal(all.length, 2);
});

function task(id: string, label: string, group: IWorkspaceTask["group"]): IWorkspaceTask {
  return Object.freeze({ id, label, group, command: `run ${id}`, source: "vscode" });
}

class FakeTaskService extends DisposableOwner implements ITaskService {
  private readonly tasksEmitter = this.own(new Emitter<readonly IWorkspaceTask[]>());
  readonly startEmitter = this.own(new Emitter<ITaskRun>());
  readonly runEmitter = this.own(new Emitter<ITaskRun>());
  readonly onDidChangeTasks = this.tasksEmitter.event;
  readonly onDidStartTask = this.startEmitter.event;
  readonly onDidChangeTaskRun = this.runEmitter.event;
  readonly runs: FakeTaskRun[] = [];
  lastRun: ITaskRun | undefined;
  constructor(readonly tasks: readonly IWorkspaceTask[]) { super(); }
  get activeRuns(): readonly ITaskRun[] { return this.runs.filter(run => run.status === "running"); }
  async refresh() { return this.tasks; }
  async run(task: IWorkspaceTask): Promise<ITaskRun> { const run = this.own(new FakeTaskRun(task)); this.runs.push(run); this.lastRun = run; this.startEmitter.fire(run); return run; }
  async terminate(run: ITaskRun) { (run as FakeTaskRun).finish("canceled"); }
}

class FakeTaskRun extends DisposableOwner implements ITaskRun {
  private readonly emitter = this.own(new Emitter<TaskRunStatus>());
  readonly onDidChangeStatus = this.emitter.event;
  readonly terminal = {} as ITerminalInstance;
  status: TaskRunStatus = "running";
  exitCode: number | undefined;
  constructor(readonly task: IWorkspaceTask) { super(); }
  finish(status: TaskRunStatus): void { this.status = status; this.exitCode = status === "failed" ? 1 : status === "succeeded" ? 0 : undefined; this.emitter.fire(status); }
}

const _noEvent = (() => toDisposable(() => undefined)) as Event<never>;
