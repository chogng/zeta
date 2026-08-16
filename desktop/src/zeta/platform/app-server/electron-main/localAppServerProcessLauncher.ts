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
  private environmentValue: Readonly<Record<string, string>>;

  readonly description: string;

  constructor(readonly options: LocalAppServerProcessLauncherOptions) {
    if (!isAbsolute(options.executable)) throw new Error("App Server executable path must be absolute");
    this.validateEnvironment(options.environment);
    this.description = options.executable;
    this.spawnProcess = options.spawnProcess ?? defaultSpawn;
    this.fileExists = options.fileExists ?? existsSync;
    this.environmentValue = options.environment;
  }

  get environment(): Readonly<Record<string, string>> {
    return this.environmentValue;
  }

  /** Selects the immutable authority scope used by the next launched connection. */
  replaceEnvironment(environment: Readonly<Record<string, string>>): void {
    this.validateEnvironment(environment);
    this.environmentValue = environment;
  }

  validate(): void {
    if (!this.fileExists(this.options.executable)) throw new Error(`Packaged Zeta binary is missing: ${this.options.executable}`);
  }

  launch(): ChildProcessWithoutNullStreams {
    return this.spawnProcess(this.options.executable, this.options.args, { environment: this.environmentValue });
  }

  private validateEnvironment(environment: Readonly<Record<string, string>>): void {
    const allowedEnvironmentKeys = this.options.allowedEnvironmentKeys ? new Set(this.options.allowedEnvironmentKeys) : undefined;
    for (const key of Object.keys(environment)) {
      const allowed = allowedEnvironmentKeys ? allowedEnvironmentKeys.has(key) : isAllowedAppServerEnvironmentKey(key);
      if (!allowed) throw new Error(`App Server environment variable is not allowed: ${key}`);
    }
  }
}

function defaultSpawn(executable: string, args: readonly string[], options: SpawnLocalAppServerOptions): ChildProcessWithoutNullStreams {
  return spawn(executable, [...args], { env: { ...options.environment }, shell: false, stdio: "pipe" });
}
