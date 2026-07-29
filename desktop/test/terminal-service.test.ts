import assert from "node:assert/strict";
import test from "node:test";
import type { TerminalCreateParams, TerminalReadResult } from "../generated/app-server/types.js";
import { toDisposable } from "../src/zeta/base/common/lifecycle.js";
import type { AppServerConnectionState } from "../src/zeta/platform/app-server/common/renderer-api.js";
import type { ITerminalBackend } from "../src/zeta/workbench/contrib/terminal/browser/appServerTerminalBackend.js";
import { TerminalService } from "../src/zeta/workbench/contrib/terminal/browser/terminalService.js";
import type { ITerminalInstance } from "../src/zeta/workbench/contrib/terminal/common/terminal.js";

const DEFAULT_PROFILE = {
  profileId: "command-prompt",
  title: "Command Prompt",
  isDefault: true,
} as const;

test("TerminalService exposes event-driven instances over the pull backend", async () => {
  const backend = new TestTerminalBackend([
    readResult({
      chunks: [{
        sequence: 1,
        dataBase64: Buffer.from("hello").toString("base64"),
      }],
      nextSequence: 1,
    }),
    readResult({
      nextSequence: 1,
      exited: true,
      exitCode: 0,
    }),
  ]);
  using service = new TerminalService(backend);
  const output: Uint8Array[] = [];
  let createdInstance: ITerminalInstance | undefined;
  service.onDidCreateInstance((instance) => {
    createdInstance = instance;
    instance.onDidWriteData((data) => output.push(data));
  });

  const instance = await service.createTerminal({
    dimensions: { rows: 24, cols: 80 },
    profile: { type: "default" },
  });
  await waitFor(() => instance.state === "exited");

  assert.equal(instance, createdInstance);
  assert.equal(service.activeInstance, instance);
  assert.equal(new TextDecoder().decode(output[0]), "hello");
  assert.equal(instance.exitCode, 0);
  assert.deepEqual(backend.createCalls, [{
    rows: 24,
    cols: 80,
    profile: { type: "default" },
  }]);
  assert.deepEqual(backend.readCursors, [0, 1]);
});

test("TerminalService batches input, coalesces resize, and releases terminals", async () => {
  const backend = new TestTerminalBackend([]);
  using service = new TerminalService(backend);
  const instance = await service.createTerminal({
    dimensions: { rows: 20, cols: 60 },
    profile: { type: "profile", profileId: "command-prompt" },
  });

  instance.write("a");
  instance.write("b");
  instance.resize({ rows: 30, cols: 100 });
  instance.resize({ rows: 31, cols: 101 });
  await waitFor(() => backend.writeCalls.length === 1 && backend.resizeCalls.length === 1);
  await service.closeTerminal(instance);

  assert.equal(backend.writeCalls[0].data, "ab");
  assert.deepEqual(backend.resizeCalls[0], {
    terminalId: "terminal-1",
    rows: 31,
    cols: 101,
  });
  assert.deepEqual(backend.closeCalls, ["terminal-1"]);
  assert.equal(service.instances.length, 0);
  assert.equal(service.activeInstance, undefined);
});

test("TerminalService keeps multiple instances and safely relaunches after a crash", async () => {
  const backend = new TestTerminalBackend([]);
  using service = new TerminalService(backend);
  const first = await service.createTerminal({
    dimensions: { rows: 24, cols: 80 },
    profile: { type: "default" },
  });
  const second = await service.createTerminal({
    dimensions: { rows: 30, cols: 100 },
    profile: { type: "profile", profileId: "command-prompt" },
  });

  assert.equal(service.instances.length, 2);
  assert.equal(service.activeInstance, second);
  service.setActiveInstance(first);
  assert.equal(service.activeInstance, first);

  backend.emitConnectionState("crashed");
  await waitFor(() => first.state === "disconnected" && second.state === "disconnected");
  backend.emitConnectionState("ready");
  assert.equal(first.state, "disconnected");
  await service.relaunchTerminal(first, { rows: 25, cols: 90 });

  assert.equal(first.state, "running");
  assert.equal(first.id, "terminal-instance-1");
  assert.equal(backend.createCalls.length, 3);
  assert.deepEqual(backend.createCalls[2], {
    rows: 25,
    cols: 90,
    profile: { type: "profile", profileId: "command-prompt" },
  });
});

class TestTerminalBackend implements ITerminalBackend {
  readonly createCalls: TerminalCreateParams[] = [];
  readonly writeCalls: Array<{ terminalId: string; data: string }> = [];
  readonly resizeCalls: Array<{ terminalId: string; rows: number; cols: number }> = [];
  readonly readCursors: number[] = [];
  readonly closeCalls: string[] = [];
  readonly #connectionListeners = new Set<(state: AppServerConnectionState) => void>();
  #connectionState: AppServerConnectionState = "ready";

  constructor(private readonly reads: TerminalReadResult[]) {}

  async listProfiles() {
    return { profiles: [DEFAULT_PROFILE] };
  }

  async create(params: TerminalCreateParams) {
    this.createCalls.push(params);
    return {
      terminalId: `terminal-${this.createCalls.length}`,
      profile: DEFAULT_PROFILE,
    };
  }

  async write(params: { terminalId: string; data: string }) {
    this.writeCalls.push(params);
  }

  async resize(params: { terminalId: string; rows: number; cols: number }) {
    this.resizeCalls.push(params);
  }

  async read(params: { terminalId: string; afterSequence: number; maxChunks: number }) {
    this.readCursors.push(params.afterSequence);
    return this.reads.shift() ?? readResult({ nextSequence: params.afterSequence });
  }

  async close(params: { terminalId: string }) {
    this.closeCalls.push(params.terminalId);
  }

  async getConnectionState(): Promise<AppServerConnectionState> {
    return this.#connectionState;
  }

  onConnectionState(listener: (state: AppServerConnectionState) => void) {
    this.#connectionListeners.add(listener);
    return toDisposable(() => this.#connectionListeners.delete(listener));
  }

  emitConnectionState(state: AppServerConnectionState): void {
    this.#connectionState = state;
    for (const listener of this.#connectionListeners) listener(state);
  }
}

function readResult(overrides: Partial<TerminalReadResult>): TerminalReadResult {
  return {
    terminalId: "terminal-1",
    chunks: [],
    nextSequence: 0,
    outputGap: false,
    exited: false,
    exitCode: null,
    ...overrides,
  };
}

async function waitFor(condition: () => boolean, timeoutMillis = 1_000): Promise<void> {
  const deadline = Date.now() + timeoutMillis;
  while (!condition()) {
    if (Date.now() >= deadline) throw new Error("Timed out waiting for terminal state");
    await new Promise((resolve) => setTimeout(resolve, 1));
  }
}
