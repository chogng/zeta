import { randomUUID } from "node:crypto";
import { APP_SERVER_METHODS, type WorkspaceSwitchTrust } from "../../../../../generated/app-server/types.js";
import type { IDisposable } from "../../../base/common/lifecycle.js";
import type { AppServerConnectionState } from "../../app-server/common/appServerApi.js";
import { AppServerRemoteError } from "../../app-server/common/appServerError.js";
import type { AppServerSupervisor } from "../../app-server/electron-main/app-server-supervisor.js";
import { type IWorkspaceRuntimeSwitcher, type IWorkspaceTransitionContext, type IWorkspaceTransitionFailure, type IWorkspaceTransitionRecoveryRouter, WorkspaceTransitionFailureKind, WorkspaceTransitionRecovery, WorkspaceTrustChoice } from "./workspaceTransitionMainService.js";

export interface IAppServerWorkspaceTransitionHost {
  getState(): AppServerConnectionState;
  switchWorkspace(root: string, trust: IWorkspaceTransitionContext["trust"]): Promise<void>;
  onStateChange(listener: (state: AppServerConnectionState) => void): IDisposable;
}

/**
 * Adapts App Server connection lifecycle into Workspace transition semantics.
 *
 * Only connection loss is retryable. Busy, unsupported protocol, and runtime
 * rejection remain visible domain failures and never trigger process restart.
 */
export class AppServerWorkspaceTransitionAdapter implements IWorkspaceRuntimeSwitcher, IWorkspaceTransitionRecoveryRouter {
  constructor(private readonly host: IAppServerWorkspaceTransitionHost) {}

  switchWorkspace({ workspace, trust }: IWorkspaceTransitionContext): Promise<void> {
    return this.host.switchWorkspace(workspace.uri.fsPath, trust);
  }

  classifyRuntimeError(error: unknown): WorkspaceTransitionFailureKind {
    if (error instanceof AppServerRemoteError) {
      switch (error.errorName) {
        case "WorkspaceSwitchBusy":
          return WorkspaceTransitionFailureKind.RuntimeBusy;
        case "WorkspaceSwitchUnavailable":
        case "MethodNotFound":
          return WorkspaceTransitionFailureKind.RuntimeUnsupported;
        default:
          return WorkspaceTransitionFailureKind.RuntimeRejected;
      }
    }
    if (
      this.host.getState() !== "ready"
      || (error instanceof Error && /connection closed|stdout ended|exited|not ready/i.test(error.message))
    ) {
      return WorkspaceTransitionFailureKind.RuntimeUnavailable;
    }
    return WorkspaceTransitionFailureKind.RuntimeRejected;
  }

  async recover(failure: IWorkspaceTransitionFailure): Promise<WorkspaceTransitionRecovery> {
    if (failure.kind !== WorkspaceTransitionFailureKind.RuntimeUnavailable) {
      return WorkspaceTransitionRecovery.KeepCurrent;
    }
    try {
      await this.waitUntilReady({ timeoutMs: 10_000 });
      return WorkspaceTransitionRecovery.Retry;
    } catch {
      return WorkspaceTransitionRecovery.KeepCurrent;
    }
  }

  private waitUntilReady(options: IWaitUntilReadyOptions): Promise<void> {
    const state = this.host.getState();
    if (state === "ready") return Promise.resolve();
    if (state === "stopped" || state === "stopping") {
      return Promise.reject(new Error(`App Server cannot recover from ${state}`));
    }
    return new Promise<void>((resolve, reject) => {
      let settled = false;
      let subscription: IDisposable | undefined;
      const finish = (error?: Error): void => {
        if (settled) return;
        settled = true;
        clearTimeout(timeout);
        subscription?.dispose();
        if (error) reject(error);
        else resolve();
      };
      const timeout = setTimeout(() => {
        finish(new Error("Timed out waiting for App Server recovery"));
      }, options.timeoutMs);
      timeout.unref();
      subscription = this.host.onStateChange((nextState) => {
        if (nextState === "ready") {
          finish();
        } else if (nextState === "stopped" || nextState === "stopping") {
          finish(new Error(`App Server recovery stopped in ${nextState}`));
        }
      });
    });
  }
}

interface IWaitUntilReadyOptions {
  readonly timeoutMs: number;
}

export function createAppServerWorkspaceTransitionAdapter(
  supervisor: AppServerSupervisor,
): AppServerWorkspaceTransitionAdapter {
  return new AppServerWorkspaceTransitionAdapter({
    getState: () => supervisor.state,
    switchWorkspace: async (root, trust) => {
      await switchAppServerWorkspace(supervisor, root, trust);
    },
    onStateChange: (listener) => supervisor.onStateChange(listener),
  });
}

export async function readAppServerWorkspaceTrust(supervisor: AppServerSupervisor, root: string): Promise<WorkspaceTrustChoice | undefined> {
  const result = await supervisor.request(APP_SERVER_METHODS["workspace/trust/read"], { root });
  return result.setting === "trusted"
    ? WorkspaceTrustChoice.Trusted
    : result.setting === "restricted" ? WorkspaceTrustChoice.Restricted : undefined;
}

export async function switchAppServerWorkspace(supervisor: AppServerSupervisor, root: string, trust: WorkspaceTrustChoice): Promise<void> {
  const authority = await workspaceSwitchTrust(supervisor, trust);
  await supervisor.request(APP_SERVER_METHODS["workspace/switch"], { root, trust: authority });
}

async function workspaceSwitchTrust(supervisor: AppServerSupervisor, trust: WorkspaceTrustChoice): Promise<WorkspaceSwitchTrust> {
  switch (trust) {
    case WorkspaceTrustChoice.UserConfig:
      return { type: "userConfig" } as const;
    case WorkspaceTrustChoice.Restricted:
    case WorkspaceTrustChoice.Trusted: {
      const config = await supervisor.request(APP_SERVER_METHODS["config/read"], {});
      return {
        type: "userDecision" as const,
        commandId: `desktop-workspace-trust-${randomUUID()}`,
        expectedRevision: config.revision,
        setting: trust,
      };
    }
  }
}
