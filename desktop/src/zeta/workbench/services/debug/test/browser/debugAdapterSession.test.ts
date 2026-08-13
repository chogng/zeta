import assert from "node:assert/strict";
import test from "node:test";
import { Emitter } from "../../../../../base/common/event.js";
import { URI } from "../../../../../base/common/uri.js";
import { type IDebugAdapterProcessReadResult, type IDebugAdapterProcessService } from "../../../../../platform/debug/common/debugAdapterProcessService.js";
import { type AppServerConnectionState } from "../../../../../platform/app-server/common/appServerApi.js";
import { DebugAdapterSession } from "../../browser/debugAdapterSession.js";
import { type IDebugBreakpoint, type IDebugConfiguration } from "../../common/debugService.js";

test("DebugAdapterSession performs DAP configuration, clears breakpoints, and resolves an omitted stopped thread", async () => {
  using processes = new FakeDebugAdapterProcessService();
  let breakpoints: readonly IDebugBreakpoint[] = [breakpoint(4)];
  const updates: Array<{ readonly id: string; readonly verified: boolean; readonly message?: string }> = [];
  const terminalRequests: unknown[] = [];
  const session = await DebugAdapterSession.start({ configuration: configuration(), processService: processes, breakpoints: () => breakpoints, workspaceFolder: "C:\\workspace", runInTerminal: async value => { terminalRequests.push(value); return {}; }, updateBreakpoints: values => updates.push(...values) });

  assert.equal(session.state, "running");
  assert.deepEqual(processes.started, { program: "C:\\workspace\\adapter", arguments: ["--stdio", "C:\\workspace"] });
  assert.deepEqual(processes.request("launch").arguments, { program: "C:\\workspace\\bin\\app", cwd: "C:\\workspace" });
  assert.deepEqual(processes.request("setBreakpoints").arguments, { source: { path: "C:\\workspace\\main.ts" }, breakpoints: [{ line: 4 }] });
  assert.deepEqual(updates, [{ id: "main:4", verified: true }]);

  processes.reverseRequest("runInTerminal", { kind: "integrated", args: ["app"] });
  await waitFor(() => processes.responses("runInTerminal").length === 1);
  assert.deepEqual(terminalRequests, [{ kind: "integrated", args: ["app"] }]);
  assert.equal(processes.responses("runInTerminal")[0]?.success, true);

  breakpoints = [];
  await session.syncBreakpoints();
  assert.deepEqual(processes.requests("setBreakpoints").at(-1)?.arguments, { source: { path: "C:\\workspace\\main.ts" }, breakpoints: [] });

  processes.event("stopped", { reason: "breakpoint", allThreadsStopped: true });
  await waitFor(() => session.state === "stopped");
  const frames = await session.stackTrace();
  assert.equal(processes.requests("threads").length, 1);
  assert.deepEqual(frames, [{ id: 11, name: "main", source: { name: "main.ts", path: "C:\\workspace\\main.ts" }, lineNumber: 4, columnNumber: 1 }]);

  await session.disconnect();
  assert.equal(processes.closed, true);
});

class FakeDebugAdapterProcessService implements IDebugAdapterProcessService {
  private readonly connectionEmitter = new Emitter<AppServerConnectionState>();
  private readonly messages: Array<{ readonly sequence: number; readonly message: unknown }> = [];
  private nextMessageSequence = 0;
  private nextProtocolSequence = 100;
  readonly sent: Array<Record<string, unknown>> = [];
  started: unknown;
  closed = false;
  readonly onConnectionState = this.connectionEmitter.event;

  async start(options: unknown): Promise<string> { this.started = options; return "debug-1"; }

  async send(_sessionId: string, message: unknown): Promise<void> {
    const request = message as Record<string, unknown>;
    this.sent.push(request);
    if (request.type !== "request") return;
    const command = String(request.command);
    if (command === "launch") this.event("initialized");
    const body = command === "initialize" ? { supportsConfigurationDoneRequest: true }
      : command === "threads" ? { threads: [{ id: 7, name: "main" }] }
      : command === "stackTrace" ? { stackFrames: [{ id: 11, name: "main", source: { name: "main.ts", path: "C:\\workspace\\main.ts" }, line: 4, column: 1 }] }
      : command === "setBreakpoints" && Array.isArray((request.arguments as Record<string, unknown>)?.breakpoints) && ((request.arguments as Record<string, unknown>).breakpoints as unknown[]).length > 0 ? { breakpoints: [{ verified: true }] }
      : {};
    this.enqueue({ seq: this.nextProtocolSequence++, type: "response", request_seq: request.seq, success: true, command, body });
  }

  async read(_sessionId: string, afterSequence: number, maxMessages: number): Promise<IDebugAdapterProcessReadResult> {
    const messages = this.messages.filter(message => message.sequence >= afterSequence).slice(0, maxMessages);
    return { messages, nextSequence: this.nextMessageSequence, outputGap: false, stderr: "", exited: false, exitCode: null, protocolError: null };
  }

  async close(): Promise<void> { this.closed = true; }
  async getConnectionState(): Promise<AppServerConnectionState> { return "ready"; }
  dispose(): void { this.connectionEmitter.dispose(); }
  [Symbol.dispose](): void { this.dispose(); }

  event(event: string, body?: unknown): void { this.enqueue({ seq: this.nextProtocolSequence++, type: "event", event, ...(body === undefined ? {} : { body }) }); }
  reverseRequest(command: string, argumentsValue: unknown): void { this.enqueue({ seq: this.nextProtocolSequence++, type: "request", command, arguments: argumentsValue }); }
  request(command: string): Record<string, unknown> { const request = this.requests(command)[0]; assert.ok(request); return request; }
  requests(command: string): Record<string, unknown>[] { return this.sent.filter(message => message.type === "request" && message.command === command); }
  responses(command: string): Record<string, unknown>[] { return this.sent.filter(message => message.type === "response" && message.command === command); }
  private enqueue(message: unknown): void { this.messages.push({ sequence: this.nextMessageSequence++, message }); }
}

function configuration(): IDebugConfiguration {
  return { id: "launch:0:test", name: "Test", type: "example", request: "launch", adapter: { program: "${workspaceFolder}\\adapter", arguments: ["--stdio", "${workspaceFolder}"] }, arguments: { program: "${workspaceFolder}\\bin\\app", cwd: "${workspaceFolder}" } };
}

function breakpoint(lineNumber: number): IDebugBreakpoint {
  return { id: `main:${lineNumber}`, resource: URI.file("C:\\workspace\\main.ts"), lineNumber, enabled: true, verified: false };
}

async function waitFor(predicate: () => boolean): Promise<void> {
  const deadline = Date.now() + 2_000;
  while (!predicate()) {
    if (Date.now() >= deadline) throw new Error("Timed out waiting for debug session state");
    await new Promise(resolve => setTimeout(resolve, 10));
  }
}
