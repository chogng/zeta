import assert from "node:assert/strict";
import test from "node:test";
import { toDisposable } from "../../../../../base/common/lifecycle.js";
import { CommandRegistry } from "../../../../../platform/commands/common/commands.js";
import type { ServicesAccessor } from "../../../../../platform/instantiation/common/instantiation.js";
import type { AppServerConnectionState } from "../../../../../platform/app-server/common/appServerApi.js";
import type { ExtensionHostFleetSnapshot, ExtensionHostInvocationRequest, ExtensionHostReconcileMode, IExtensionHostApi, JsonValue } from "../../../../../platform/extensionHost/common/extensionHostApi.js";
import { TextModel } from "../../../../../editor/common/model/textModel.js";
import { TextPosition } from "../../../../../editor/common/core/text.js";
import { LanguageFeaturesService } from "../../../language/common/languageFeaturesService.js";
import type { ITaskService, TaskProvider, TaskProviderRegistration } from "../../../tasks/common/taskService.js";
import type { ITestingService, TestProfileProvider, TestProfileProviderRegistration } from "../../../testing/common/testingService.js";
import { AppServerExtensionHostService } from "../../browser/appServerExtensionHostService.js";

const DIGEST = `sha256:${"b".repeat(64)}`;

test("keeps last-good contributions while refreshing and revokes them synchronously on disconnect", async () => {
  const api = new FakeExtensionHostApi(snapshot(1, "acme.old"));
  const commands = new CommandRegistry();
  const tasks = new ProviderSink<TaskProvider>();
  const tests = new ProviderSink<TestProfileProvider>();
  using languages = new LanguageFeaturesService();
  using service = new AppServerExtensionHostService({ api, commands, languageFeatures: languages, tasks: tasks as unknown as ITaskService, testing: tests as unknown as ITestingService, invocationTimeoutMillis: 1_000 });
  const failures: string[] = [];
  service.onDidFail(failure => failures.push(failure.code));

  await service.start();
  assert.equal(service.currentSnapshot.fleetGeneration, 1);
  assert.equal(service.state, "degraded");
  assert.ok(failures.includes("unsupportedRegistrationBridge"));
  assert.ok(commands.hasCommand("acme.old"));

  const handler = commands.getCommand("acme.old")!;
  assert.deepEqual(await handler({} as ServicesAccessor, "argument"), { executed: true });
  assert.equal(api.invocations[0]?.activationGeneration, 11);
  assert.equal(api.invocations[0]?.incarnation, 3);
  const payload = api.invocations[0]?.payload;
  assert.deepEqual(typeof payload === "object" && payload !== null && !Array.isArray(payload) ? (payload as { readonly arguments?: JsonValue }).arguments : undefined, ["argument"]);

  assert.deepEqual(await tasks.providers[0]!.provideTasks(new AbortController().signal), [{ id: "unit", label: "Unit", command: "pnpm test", group: "test" }]);
  assert.deepEqual(await tests.providers[0]!.provideTestProfiles(new AbortController().signal), [{ id: "unit", label: "Unit", taskId: "extension:extensionHost.61636d652e64656d6f.7461736b73:unit" }]);

  const pendingList = deferred<ExtensionHostFleetSnapshot>();
  api.listResult = pendingList.promise;
  api.emitChanged(2);
  assert.equal(commands.hasCommand("acme.old"), true);
  assert.equal(tasks.providers.length, 1);
  api.current = snapshot(2, "acme.new");
  pendingList.resolve(api.current);
  await waitFor(() => service.currentSnapshot.fleetGeneration === 2);
  assert.ok(commands.hasCommand("acme.new"));

  api.listResult = Promise.reject(new Error("temporary list failure"));
  api.emitChanged(3);
  await waitFor(() => failures.includes("extensionHostRefreshFailed"));
  assert.equal(service.currentSnapshot.fleetGeneration, 2);
  assert.ok(commands.hasCommand("acme.new"));
  assert.equal(service.state, "degraded");
  api.current = snapshot(3, "acme.latest");
  api.listResult = undefined;

  api.emitConnection("crashed");
  assert.equal(commands.hasCommand("acme.new"), false);
  assert.equal(service.state, "failed");
  api.emitConnection("ready");
  await waitFor(() => commands.hasCommand("acme.latest"));
});

test("projects supported language operations while diagnosing unsupported operations", async () => {
  const current = snapshot(1, "acme.run", [{ registrationId: "language", kind: "languageProvider", languageIds: ["typescript"], operations: ["hover", "parameterHints", "definition"] }]);
  const api = new FakeExtensionHostApi(current);
  const tasks = new ProviderSink<TaskProvider>();
  const tests = new ProviderSink<TestProfileProvider>();
  using languages = new LanguageFeaturesService();
  using service = new AppServerExtensionHostService({ api, commands: new CommandRegistry(), languageFeatures: languages, tasks: tasks as unknown as ITaskService, testing: tests as unknown as ITestingService });
  const failures: string[] = [];
  service.onDidFail(failure => failures.push(failure.code));
  await service.start();

  using model = new TextModel("answer");
  using hover = languages.createHoverService(model);
  using parameterHints = languages.createParameterHintsService(model);
  assert.deepEqual(await hover.provideHover("typescript", TextPosition.at(0, 1)), { contents: ["Host hover"] });
  assert.deepEqual(await parameterHints.provideParameterHints("typescript", TextPosition.at(0, 1)), { signatures: [{ label: "fn(value)", parameters: [{ label: "value" }], activeParameter: 0 }], activeSignature: 0 });
  assert.equal(service.state, "degraded");
  assert.ok(failures.includes("unsupportedRegistrationBridge"));
});

test("keeps the service stopped when the negotiated Host capability is absent", async () => {
  const api = new FakeExtensionHostApi(snapshot(1, "acme.run"));
  api.available = false;
  const tasks = new ProviderSink<TaskProvider>();
  const tests = new ProviderSink<TestProfileProvider>();
  using languages = new LanguageFeaturesService();
  using service = new AppServerExtensionHostService({ api, commands: new CommandRegistry(), languageFeatures: languages, tasks: tasks as unknown as ITaskService, testing: tests as unknown as ITestingService });
  const failures: string[] = [];
  service.onDidFail(failure => failures.push(failure.code));

  await service.start();
  assert.equal(service.state, "stopped");
  assert.equal(service.currentSnapshot.fleetGeneration, 0);
  assert.equal(api.reconciles, 0);
  assert.deepEqual(failures, []);
});

class FakeExtensionHostApi implements IExtensionHostApi {
  available = true;
  reconciles = 0;
  current: ExtensionHostFleetSnapshot;
  listResult: Promise<ExtensionHostFleetSnapshot> | undefined;
  readonly invocations: ExtensionHostInvocationRequest[] = [];
  private readonly changed = new Set<(generation: number) => void>();
  private readonly connections = new Set<(state: AppServerConnectionState) => void>();
  private connectionState: AppServerConnectionState = "ready";

  constructor(snapshotValue: ExtensionHostFleetSnapshot) { this.current = snapshotValue; }

  list(): Promise<ExtensionHostFleetSnapshot> { return this.listResult ?? Promise.resolve(this.current); }
  reconcile(_mode: ExtensionHostReconcileMode): Promise<ExtensionHostFleetSnapshot> { this.reconciles += 1; return Promise.resolve(this.current); }
  isAvailable(): Promise<boolean> { return Promise.resolve(this.available); }
  getConnectionState(): Promise<AppServerConnectionState> { return Promise.resolve(this.connectionState); }
  onDidChange(listener: (generation: number) => void) { this.changed.add(listener); return { dispose: () => this.changed.delete(listener) }; }
  onConnectionState(listener: (state: AppServerConnectionState) => void) { this.connections.add(listener); return { dispose: () => this.connections.delete(listener) }; }

  async invoke(request: ExtensionHostInvocationRequest, signal: AbortSignal): Promise<JsonValue> {
    signal.throwIfAborted();
    this.invocations.push(request);
    if (request.operation === "execute") return Object.freeze({ executed: true });
    if (request.operation === "provideTasks") return Object.freeze({ tasks: Object.freeze([{ id: "unit", label: "Unit", command: "pnpm test", group: "test" }]) });
    if (request.operation === "provideTestProfiles") return Object.freeze({ profiles: Object.freeze([{ id: "unit", label: "Unit", taskProviderRegistrationId: "tasks", taskId: "unit" }]) });
    if (request.operation === "hover") return Object.freeze({ contents: Object.freeze(["Host hover"]) });
    if (request.operation === "parameterHints") return Object.freeze({ signatures: Object.freeze([{ label: "fn(value)", parameters: Object.freeze([{ label: "value" }]), activeParameter: 0 }]), activeSignature: 0 });
    throw new Error(`Unexpected operation ${request.operation}`);
  }

  emitChanged(generation: number): void { for (const listener of this.changed) listener(generation); }
  emitConnection(state: AppServerConnectionState): void { this.connectionState = state; for (const listener of this.connections) listener(state); }
}

class ProviderSink<TProvider> {
  providers: readonly TProvider[] = Object.freeze([]);

  registerTaskProviders(providers: readonly TaskProvider[]): TaskProviderRegistration { return this.registration(providers as readonly TProvider[]) as TaskProviderRegistration; }
  registerTestProfileProviders(providers: readonly TestProfileProvider[]): TestProfileProviderRegistration { return this.registration(providers as readonly TProvider[]) as TestProfileProviderRegistration; }

  private registration(initial: readonly TProvider[]): TaskProviderRegistration | TestProfileProviderRegistration {
    let disposed = false;
    this.providers = Object.freeze([...initial]);
    const registration = toDisposable(() => { disposed = true; this.providers = Object.freeze([]); }) as TaskProviderRegistration;
    registration.replace = providers => {
      if (disposed) throw new ReferenceError("Provider registration is disposed");
      this.providers = Object.freeze([...(providers as readonly TProvider[])]);
    };
    return registration;
  }
}

function snapshot(generation: number, command: string, additional: readonly ExtensionHostFleetSnapshot["extensions"][number]["registrations"][number][] = []): ExtensionHostFleetSnapshot {
  return Object.freeze({
    generation,
    extensions: Object.freeze([Object.freeze({
      id: "acme.demo",
      version: "1.0.0",
      packageDigest: DIGEST,
      runtimeApiVersion: 1,
      activationGeneration: 10 + generation,
      incarnation: 3,
      lifecycle: "ready" as const,
      failure: undefined,
      registrations: Object.freeze([
        Object.freeze({ registrationId: "command", kind: "command" as const, command, title: "Run" }),
        Object.freeze({ registrationId: "tasks", kind: "taskProvider" as const, taskType: "acme" }),
        Object.freeze({ registrationId: "tests", kind: "testProfileProvider" as const, providerId: "acme.tests", label: "Tests" }),
        Object.freeze({ registrationId: "debug", kind: "debugAdapter" as const, debuggerType: "acme" }),
        ...additional,
      ]),
    })]),
  });
}

function deferred<T>(): { readonly promise: Promise<T>; resolve(value: T): void } {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>(accept => { resolve = accept; });
  return { promise, resolve };
}

async function waitFor(predicate: () => boolean): Promise<void> {
  for (let attempt = 0; attempt < 100; attempt += 1) {
    if (predicate()) return;
    await new Promise(resolve => setTimeout(resolve, 0));
  }
  throw new Error("Timed out waiting for Extension Host state");
}
