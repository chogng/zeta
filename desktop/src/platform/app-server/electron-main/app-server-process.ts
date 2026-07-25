import { ChildProcessWithoutNullStreams, spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { join } from "node:path";
import { app } from "electron";
import { JsonRpcConnection } from "./json-rpc-connection.js";

/** Starts only the packaged Zeta executable and exposes its typed JSON-RPC connection. */
export class AppServerProcess {
  #process?: ChildProcessWithoutNullStreams;
  #connection?: JsonRpcConnection;

  start(): JsonRpcConnection {
    if (this.#connection) return this.#connection;
    const executable = join(process.resourcesPath, "bin", "zeta");
    if (!existsSync(executable)) throw new Error(`Packaged Zeta binary is missing: ${executable}`);
    const environment = { PATH: process.env.PATH ?? "", ZETA_STATE_ROOT: join(app.getPath("userData"), "state") };
    this.#process = spawn(executable, ["app-server", "--listen", "stdio://"], { env: environment, shell: false, stdio: "pipe" });
    this.#connection = new JsonRpcConnection(this.#process);
    return this.#connection;
  }

  stop(): void {
    this.#process?.kill();
    this.#process = undefined;
    this.#connection = undefined;
  }
}
