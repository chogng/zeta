import { type ChildProcessWithoutNullStreams, spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { isAbsolute } from "node:path";
import { isAllowedAppServerEnvironmentKey } from "../common/appServerEnvironment.js";
import type { IAppServerProcessLauncher } from "./appServerProcessLauncher.js";

export interface SpawnLocalAppServerOptions {
  readonly environment: Readonly<Record<string, string>>;
}

export type SpawnLocalAppServer = (executable: string, args: readonly string[], options: SpawnLocalAppServerOptions) => ChildProcessWithoutNullStreams;

export interface LocalAppServerProcessLauncherOptions {
  readonly executable: string;
  readonly args: readonly string[];
  readonly environment: Readonly<Record<string, string>>;
  readonly allowedEnvironmentKeys?: readonly string[];
  readonly spawnProcess?: SpawnLocalAppServer;
  readonly fileExists?: (path: string) => boolean;
}

/** Launches the packaged App Server directly on the Desktop host. */
export class LocalAppServerProcessLauncher implements IAppServerProcessLauncher {
  private readonly spawnProcess: SpawnLocalAppServer;
  private readonly fileExists: (path: string) => boolean;

  readonly description: string;

  constructor(readonly options: LocalAppServerProcessLauncherOptions) {
    if (!isAbsolute(options.executable)) throw new Error("App Server executable path must be absolute");
    const allowedEnvironmentKeys = options.allowedEnvironmentKeys ? new Set(options.allowedEnvironmentKeys) : undefined;
    for (const key of Object.keys(options.environment)) {
      const allowed = allowedEnvironmentKeys ? allowedEnvironmentKeys.has(key) : isAllowedAppServerEnvironmentKey(key);
      if (!allowed) throw new Error(`App Server environment variable is not allowed: ${key}`);
    }
    this.description = options.executable;
    this.spawnProcess = options.spawnProcess ?? defaultSpawn;
    this.fileExists = options.fileExists ?? existsSync;
  }

  validate(): void {
    if (!this.fileExists(this.options.executable)) throw new Error(`Packaged Zeta binary is missing: ${this.options.executable}`);
  }

  launch(): ChildProcessWithoutNullStreams {
    return this.spawnProcess(this.options.executable, this.options.args, { environment: this.options.environment });
  }
}

function defaultSpawn(executable: string, args: readonly string[], options: SpawnLocalAppServerOptions): ChildProcessWithoutNullStreams {
  return spawn(executable, [...args], { env: { ...options.environment }, shell: false, stdio: "pipe" });
}
