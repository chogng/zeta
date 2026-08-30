import { randomUUID } from "node:crypto";
import { APP_SERVER_METHODS, type PermissionDto, type DirGrantDto, type EnvDirSetEntry } from "../../../../../generated/app-server/types.js";
import type { IDisposable } from "../../../base/common/lifecycle.js";
import type { AppServerConnectionState } from "../../app-server/common/appServerApi.js";
import { AppServerRemoteError } from "../../app-server/common/appServerError.js";
import type { AppServerSupervisor } from "../../app-server/electron-main/app-server-supervisor.js";
import { type IWorkspaceRuntimeSwitcher, type IWorkspaceTransitionContext, type IWorkspaceTransitionFailure, type IWorkspaceTransitionRecoveryRouter, WorkspaceTransitionFailureKind, WorkspaceTransitionRecovery } from "./workspaceTransitionMainService.js";

export const READ_DIR_PERMISSIONS: readonly PermissionDto[] = [
	"readFiles", "watchFiles", "browseFiles", "searchFiles", "inspectRepository",
];

export const DEVELOPMENT_DIR_PERMISSIONS: readonly PermissionDto[] = [
	...READ_DIR_PERMISSIONS,
	"writeFiles", "executeCommands", "loadInstructions", "loadConfig", "discoverSkills",
	"discoverMcp", "useLanguageServices", "discoverHooks", "discoverPlugins", "mutateRepository",
];

export interface IAppServerWorkspaceTransitionHost {
	getState(): AppServerConnectionState;
	switchWorkspace(root: string, grant: IWorkspaceTransitionContext["grant"]): Promise<void>;
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

	switchWorkspace({ root, grant }: IWorkspaceTransitionContext): Promise<void> {
		return this.host.switchWorkspace(root, grant);
	}

	classifyRuntimeError(error: unknown): WorkspaceTransitionFailureKind {
		if (error instanceof AppServerRemoteError) {
			switch (error.errorName) {
				case "EnvCwdSetBusy":
					return WorkspaceTransitionFailureKind.RuntimeBusy;
				case "EnvCwdSetUnavailable":
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
	switchWorkspace: (root: string, grant: DirGrantDto) => Promise<void> = async (root, grant) => {
		await switchAppServerWorkspace(supervisor, root, grant);
	},
): AppServerWorkspaceTransitionAdapter {
	return new AppServerWorkspaceTransitionAdapter({
		getState: () => supervisor.state,
		switchWorkspace,
		onStateChange: (listener) => supervisor.onStateChange(listener),
	});
}

export async function readAppServerDirPermissions(supervisor: AppServerSupervisor, path: string): Promise<readonly PermissionDto[] | undefined> {
	const result = await supervisor.request(APP_SERVER_METHODS["config/dirPermissions/read"], { path });
	return result.permissions ?? undefined;
}

export async function createUserDirGrant(supervisor: AppServerSupervisor, permissions: readonly PermissionDto[]): Promise<DirGrantDto> {
	const config = await supervisor.request(APP_SERVER_METHODS["config/read"], {});
	return {
		type: "user",
		commandId: `desktop-dir-permissions-${randomUUID()}`,
		expectedRevision: config.revision,
		permissions: [...permissions],
	};
}

export async function switchAppServerWorkspace(supervisor: AppServerSupervisor, path: string, grant: DirGrantDto): Promise<void> {
	await supervisor.request(APP_SERVER_METHODS["env/cwd/set"], { cwd: path });
	await supervisor.request(APP_SERVER_METHODS["env/dirs/set"], { dirs: [{ id: "root", path, grant }] });
}

export interface IAppServerWorkspaceFolder {
	readonly id: string;
	readonly path: string;
	readonly grant: DirGrantDto;
}

/** Atomically replaces the App Server's ordered workspace-folder collection. */
export async function setAppServerWorkspaceFolders(supervisor: AppServerSupervisor, folders: readonly IAppServerWorkspaceFolder[]): Promise<void> {
	const entries: EnvDirSetEntry[] = [];
	for (const folder of folders) {
		entries.push({ id: folder.id, path: folder.path, grant: folder.grant });
	}
	await supervisor.request(APP_SERVER_METHODS["env/dirs/set"], { dirs: entries });
}
