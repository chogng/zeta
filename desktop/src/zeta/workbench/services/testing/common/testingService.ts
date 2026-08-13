import { type Event } from "../../../../base/common/event.js";
import { type IDisposable } from "../../../../base/common/lifecycle.js";
import { createServiceIdentifier } from "../../../../platform/instantiation/common/instantiation.js";
import { type ITaskRun } from "../../tasks/common/taskService.js";

export type TestRunStatus = "running" | "completed" | "passed" | "failed" | "canceled";

export interface ITestProfile {
  readonly id: string;
  readonly label: string;
  readonly source: string;
  readonly detail?: string;
}

export interface ITestRun {
  readonly profile: ITestProfile;
  readonly taskRun: ITaskRun;
  readonly status: TestRunStatus;
  readonly onDidChangeStatus: Event<TestRunStatus>;
}

/** Test workflow over explicit test-group workspace tasks. */
export interface ITestingService extends IDisposable {
  readonly profiles: readonly ITestProfile[];
  readonly runs: readonly ITestRun[];
  readonly onDidChangeProfiles: Event<readonly ITestProfile[]>;
  readonly onDidStartRun: Event<ITestRun>;
  readonly onDidChangeRun: Event<ITestRun>;

  refresh(): Promise<readonly ITestProfile[]>;
  run(profile: ITestProfile): Promise<ITestRun>;
  runAll(): Promise<readonly ITestRun[]>;
  rerun(run: ITestRun): Promise<ITestRun>;
  cancel(run: ITestRun): Promise<void>;
}

export const ITestingService = createServiceIdentifier<ITestingService>("testingService");
