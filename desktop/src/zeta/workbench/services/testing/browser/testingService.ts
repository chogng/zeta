import { Emitter, type Event } from "../../../../base/common/event.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type ITaskRun, type ITaskService } from "../../tasks/common/taskService.js";
import { type ITestProfile, type ITestRun, type ITestingService, type TestRunStatus } from "../common/testingService.js";

/** Projects test-group workspace tasks into a dedicated testing workflow. */
export class TestingService extends DisposableOwner implements ITestingService {
  private readonly profilesEmitter = this.own(new Emitter<readonly ITestProfile[]>());
  private readonly startRunEmitter = this.own(new Emitter<ITestRun>());
  private readonly changeRunEmitter = this.own(new Emitter<ITestRun>());
  private currentProfiles: readonly ITestProfile[] = Object.freeze([]);
  private currentRuns: TestRun[] = [];

  readonly onDidChangeProfiles: Event<readonly ITestProfile[]> = this.profilesEmitter.event;
  readonly onDidStartRun: Event<ITestRun> = this.startRunEmitter.event;
  readonly onDidChangeRun: Event<ITestRun> = this.changeRunEmitter.event;

  constructor(private readonly taskService: ITaskService) {
    super();
    this.own(taskService.onDidChangeTasks(() => this.projectProfiles()));
    this.defer(() => {
      for (const run of this.currentRuns) run.dispose();
      this.currentRuns = [];
    });
    this.projectProfiles();
  }

  get profiles(): readonly ITestProfile[] { return this.currentProfiles; }
  get runs(): readonly ITestRun[] { return this.currentRuns; }

  async refresh(): Promise<readonly ITestProfile[]> {
    await this.taskService.refresh();
    this.projectProfiles();
    return this.currentProfiles;
  }

  async run(profile: ITestProfile): Promise<ITestRun> {
    const task = this.taskService.tasks.find(candidate => candidate.id === profile.id && candidate.group === "test");
    if (!task) throw new Error("Test profile is no longer present in the current workspace");
    const taskRun = await this.taskService.run(task);
    const run = this.own(new TestRun(profile, taskRun, current => this.changeRunEmitter.fire(current)));
    this.currentRuns = [...this.currentRuns, run].slice(-50);
    this.startRunEmitter.fire(run);
    return run;
  }

  async runAll(): Promise<readonly ITestRun[]> {
    const profiles = await this.refresh();
    const runs: ITestRun[] = [];
    for (const profile of profiles) runs.push(await this.run(profile));
    return runs;
  }

  rerun(run: ITestRun): Promise<ITestRun> {
    return this.run(run.profile);
  }

  cancel(run: ITestRun): Promise<void> {
    return this.taskService.terminate(run.taskRun);
  }

  private projectProfiles(): void {
    const profiles = Object.freeze(this.taskService.tasks.filter(task => task.group === "test").map(task => Object.freeze({ id: task.id, label: task.label, source: task.source, detail: task.detail ?? task.command })));
    if (JSON.stringify(profiles) === JSON.stringify(this.currentProfiles)) return;
    this.currentProfiles = profiles;
    this.profilesEmitter.fire(profiles);
  }
}

class TestRun extends DisposableOwner implements ITestRun {
  private readonly statusEmitter = this.own(new Emitter<TestRunStatus>());
  private _status: TestRunStatus;
  readonly onDidChangeStatus: Event<TestRunStatus> = this.statusEmitter.event;

  constructor(readonly profile: ITestProfile, readonly taskRun: ITaskRun, private readonly onChange: (run: TestRun) => void) {
    super();
    this._status = projectTestStatus(taskRun.status);
    this.own(taskRun.onDidChangeStatus(() => {
      const status = projectTestStatus(taskRun.status);
      if (status === this._status) return;
      this._status = status;
      this.statusEmitter.fire(status);
      this.onChange(this);
    }));
  }

  get status(): TestRunStatus { return this._status; }
}

function projectTestStatus(status: ITaskRun["status"]): TestRunStatus {
  if (status === "succeeded") return "passed";
  return status;
}
