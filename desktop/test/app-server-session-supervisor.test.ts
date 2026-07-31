import { strict as assert } from "node:assert";
import type { ChildProcessWithoutNullStreams } from "node:child_process";
import { EventEmitter } from "node:events";
import test from "node:test";
import { PassThrough } from "node:stream";
import {
  APP_SERVER_METHODS,
  APP_SERVER_SCHEMA_HASH,
} from "../generated/app-server/types.js";
import { AppServerClient } from "../src/zeta/platform/app-server/electron-main/app-server-client.js";
import { AppServerSession } from "../src/zeta/platform/app-server/electron-main/app-server-session.js";
import {
  AppServerSupervisor,
  type AppServerSupervisorOptions,
} from "../src/zeta/platform/app-server/electron-main/app-server-supervisor.js";
import { JsonRpcPeer } from "../src/zeta/platform/app-server/electron-main/json-rpc-peer.js";

class ProtocolChildProcess extends EventEmitter {
  readonly stdin = new PassThrough();
  readonly stdout = new PassThrough();
  readonly stderr = new PassThrough();
  readonly requests: Array<Record<string, unknown>> = [];
  exitCode: number | null = null;
  signalCode: NodeJS.Signals | null = null;
  private stdinBuffer = "";

  constructor(
    readonly schemaHash: string = APP_SERVER_SCHEMA_HASH,
    readonly respondToInitialize = true,
    readonly serverName = "zeta-test",
  ) {
    super();
    this.stdin.on("data", (chunk: Buffer) => this.onStdin(chunk));
  }

  kill(signal: NodeJS.Signals = "SIGTERM"): boolean {
    if (this.exitCode !== null || this.signalCode !== null) return false;
    this.signalCode = signal;
    queueMicrotask(() => this.emit("exit", null, signal));
    return true;
  }

  crash(): void {
    if (this.exitCode !== null || this.signalCode !== null) return;
    this.exitCode = 1;
    this.emit("exit", 1, null);
  }

  respond(id: unknown, result: unknown): void {
    this.stdout.write(`${JSON.stringify({ jsonrpc: "2.0", id, result })}\n`);
  }

  private onStdin(chunk: Buffer): void {
    this.stdinBuffer += chunk.toString("utf8");
    const frames = this.stdinBuffer.split("\n");
    this.stdinBuffer = frames.pop() ?? "";
    for (const frame of frames) {
      const request = JSON.parse(frame) as Record<string, unknown>;
      this.requests.push(request);
      if (request.method === "initialize" && this.respondToInitialize) {
        this.respond(request.id, {
          serverInfo: { name: this.serverName, version: "1" },
          schemaHash: this.schemaHash,
          capabilities: {
            sessions: true,
            threads: true,
            turns: true,
            resources: true,
            fileSystem: true,
            workspaceSearch: true,
            terminal: true,
            typst: true,
            updateReplay: true,
          },
        });
      } else if (request.method === "workspace/switch") {
        const params = request.params as { readonly root: string };
        this.respond(request.id, { root: params.root });
      }
    }
  }
}

function session(
  child: ProtocolChildProcess,
  options: { initializeTimeoutMs?: number } = {},
): AppServerSession {
  return new AppServerSession(
    new AppServerClient(
      new JsonRpcPeer(child as unknown as ChildProcessWithoutNullStreams),
    ),
    {
      clientName: "desktop-test",
      clientVersion: "1",
      schemaHash: APP_SERVER_SCHEMA_HASH,
      initializeTimeoutMs: options.initializeTimeoutMs ?? 100,
      expectedServerName: "zeta-test",
    },
  );
}

function supervisorOptions(
  children: ProtocolChildProcess[],
): AppServerSupervisorOptions {
  return {
    executable: "/test/zeta",
    args: ["app-server", "--listen", "stdio://"],
    environment: {
      PATH: "/test/bin",
      ZETA_RG_PATH: "/test/bin/rg",
      ZETA_STATE_ROOT: "/test/state",
    },
    session: {
      clientName: "desktop-test",
      clientVersion: "1",
      schemaHash: APP_SERVER_SCHEMA_HASH,
      initializeTimeoutMs: 100,
      expectedServerName: "zeta-test",
    },
    fileExists: () => true,
    wait: async () => {},
    spawnProcess: () => {
      const child = new ProtocolChildProcess();
      children.push(child);
      return child as unknown as ChildProcessWithoutNullStreams;
    },
  };
}

test("session becomes ready only after protocol and schema gates pass", async () => {
  const child = new ProtocolChildProcess();
  const appServer = session(child);

  const initialized = await appServer.initialize();

  assert.equal(appServer.state, "ready");
  assert.equal(initialized.schemaHash, APP_SERVER_SCHEMA_HASH);
  assert.equal(appServer.capabilities.resources, true);
  assert.equal(appServer.serverInfo.name, "zeta-test");
  await appServer.close();
  assert.equal(appServer.state, "closed");
});

test("session closes a schema-mismatched connection", async () => {
  const child = new ProtocolChildProcess("sha256:wrong");
  const appServer = session(child);

  await assert.rejects(appServer.initialize(), /schema mismatch/);

  assert.equal(appServer.state, "closed");
  assert.equal(child.signalCode, "SIGTERM");
});

test("session initialization deadline closes an unresponsive child", async () => {
  const child = new ProtocolChildProcess(APP_SERVER_SCHEMA_HASH, false);
  const appServer = session(child, { initializeTimeoutMs: 5 });

  await assert.rejects(appServer.initialize(), /timed out/);

  assert.equal(appServer.state, "closed");
  assert.equal(child.signalCode, "SIGTERM");
});

test("session rejects an unexpected server identity", async () => {
  const child = new ProtocolChildProcess(
    APP_SERVER_SCHEMA_HASH,
    true,
    "not-zeta",
  );
  const appServer = session(child);

  await assert.rejects(appServer.initialize(), /Unexpected App Server identity/);

  assert.equal(appServer.state, "closed");
});

test("supervisor restarts a crashed process with bounded lifecycle states", async () => {
  const children: ProtocolChildProcess[] = [];
  const supervisor = new AppServerSupervisor(supervisorOptions(children));
  const states: string[] = [];
  supervisor.onStateChange((state) => states.push(state));
  await supervisor.start();
  assert.equal(supervisor.state, "ready");

  const restarted = new Promise<void>((resolve) => {
    const dispose = supervisor.onStateChange((state) => {
      if (state === "ready" && children.length === 2) {
        dispose.dispose();
        resolve();
      }
    });
  });
  children[0].crash();
  await restarted;

  assert.equal(children.length, 2);
  assert.ok(states.includes("crashed"));
  assert.ok(states.includes("restarting"));
  assert.equal(supervisor.state, "ready");
  await supervisor.stop();
  assert.equal(supervisor.state, "stopped");
});

test("workspace switching keeps the current App Server process and connection", async () => {
  const children: ProtocolChildProcess[] = [];
  const supervisor = new AppServerSupervisor(supervisorOptions(children));
  const states: string[] = [];
  supervisor.onStateChange((state) => states.push(state));
  await supervisor.start();

  const switched = await supervisor.request(APP_SERVER_METHODS["workspace/switch"], {
    root: "/test/workspace",
  });

  assert.deepEqual(switched, { root: "/test/workspace" });
  assert.equal(children.length, 1);
  assert.equal(children[0].signalCode, null);
  assert.equal(supervisor.state, "ready");
  assert.deepEqual(states, ["starting", "initializing", "ready"]);
  await supervisor.stop();
});

test("crash rejects an unknown-outcome side effect without replaying it", async () => {
  const children: ProtocolChildProcess[] = [];
  const supervisor = new AppServerSupervisor(supervisorOptions(children));
  await supervisor.start();

  const turn = supervisor.request(APP_SERVER_METHODS["turn/start"], {
    commandId: "one",
    sessionId: "session_1",
    threadId: "thread_1",
    expectedSequence: 1,
    input: [{ type: "text", text: "hello" }],
  });
  await new Promise<void>((resolve) => setImmediate(resolve));
  assert.equal(children[0].requests.at(-1)?.method, "turn/start");
  const restarted = new Promise<void>((resolve) => {
    const dispose = supervisor.onStateChange((state) => {
      if (state === "ready" && children.length === 2) {
        dispose.dispose();
        resolve();
      }
    });
  });
  children[0].crash();

  await assert.rejects(turn, /exited with code 1/);
  await restarted;
  assert.deepEqual(
    children[1].requests.map((request) => request.method),
    ["initialize"],
  );
  await supervisor.stop();
});

test("supervisor stops restarting after its crash budget is exhausted", async () => {
  const children: ProtocolChildProcess[] = [];
  const options = supervisorOptions(children);
  options.maxRestartAttempts = 1;
  const supervisor = new AppServerSupervisor(options);
  await supervisor.start();

  const restarted = new Promise<void>((resolve) => {
    const dispose = supervisor.onStateChange((state) => {
      if (state === "ready" && children.length === 2) {
        dispose.dispose();
        resolve();
      }
    });
  });
  children[0].crash();
  await restarted;
  children[1].crash();
  await new Promise<void>((resolve) => setImmediate(resolve));

  assert.equal(children.length, 2);
  assert.equal(supervisor.state, "crashed");
  await supervisor.stop();
});

test("supervisor requires an absolute executable and an explicit environment allowlist", () => {
  const children: ProtocolChildProcess[] = [];
  assert.throws(
    () =>
      new AppServerSupervisor({
        ...supervisorOptions(children),
        executable: "relative/zeta",
      }),
    /must be absolute/,
  );
  assert.throws(
    () =>
      new AppServerSupervisor({
        ...supervisorOptions(children),
        environment: {
          PATH: "/test/bin",
          ZETA_STATE_ROOT: "/test/state",
          HOME: "/should-not-leak",
        },
      }),
    /HOME/,
  );
});

test("initialization failures consume exactly the bounded startup retry budget", async () => {
  const children: ProtocolChildProcess[] = [];
  const options = supervisorOptions(children);
  options.maxRestartAttempts = 1;
  options.session = { ...options.session, initializeTimeoutMs: 5 };
  options.spawnProcess = () => {
    const child = new ProtocolChildProcess(APP_SERVER_SCHEMA_HASH, false);
    children.push(child);
    return child as unknown as ChildProcessWithoutNullStreams;
  };
  const supervisor = new AppServerSupervisor(options);

  await assert.rejects(supervisor.start(), /timed out/);

  assert.equal(children.length, 2);
  assert.equal(supervisor.state, "crashed");
  await supervisor.stop();
});

test("supervisor can retry a failed startup gate after stopping", async () => {
  const children: ProtocolChildProcess[] = [];
  const options = supervisorOptions(children);
  options.maxRestartAttempts = 0;
  options.session = { ...options.session, initializeTimeoutMs: 5 };
  let respondToInitialize = false;
  options.spawnProcess = () => {
    const child = new ProtocolChildProcess(
      APP_SERVER_SCHEMA_HASH,
      respondToInitialize,
    );
    children.push(child);
    return child as unknown as ChildProcessWithoutNullStreams;
  };
  const supervisor = new AppServerSupervisor(options);

  await assert.rejects(supervisor.start(), /timed out/);
  assert.equal(supervisor.state, "crashed");

  await supervisor.stop();
  respondToInitialize = true;
  await supervisor.start();

  assert.equal(supervisor.state, "ready");
  assert.equal(children.length, 2);
  await supervisor.stop();
});
