import assert from "node:assert/strict";
import test from "node:test";
import type { ServerNotification, Session as SessionDto } from "../../../../../../../generated/app-server/types.js";
import type { IServerEventApi } from "../../../../../platform/app-server/common/appServerApi.js";
import type { ISessionApi } from "../../../../../platform/sessions/common/sessionApi.js";
import { WorkbenchSessionService } from "../../../../../workbench/services/sessions/browser/sessionService.js";

test("WorkbenchSessionService refreshes subscribed Sessions from canonical update snapshots", async () => {
  let current = session(1);
  const listeners = new Set<(event: ServerNotification) => void>();
  const subscribed: string[] = [];
  const unsubscribed: string[] = [];
  const api: ISessionApi = {
    async create() { return { session: current }; },
    async read() { return { session: current }; },
    async list() { return { sessions: [current] }; },
    async subscribe(params) {
      subscribed.push(params.sessionId);
      return { session: current, updates: [], threadProjections: [] };
    },
    async unsubscribe(params) { unsubscribed.push(params.sessionId); },
    async createThread() { throw new Error("Not used"); },
    async forkThread() { throw new Error("Not used"); },
    async archiveThread() { throw new Error("Not used"); },
    async complete() { throw new Error("Not used"); },
    async archive() { throw new Error("Not used"); },
    async stop() { throw new Error("Not used"); },
    async setModel() { throw new Error("Not used"); },
  };
  const events: IServerEventApi = {
    subscribe(listener) {
      listeners.add(listener);
      return { dispose: () => { listeners.delete(listener); } };
    },
  };
  using service = new WorkbenchSessionService({ session: api, events });
  await service.initialize();

  assert.deepEqual(subscribed, ["session-1"]);
  assert.equal(service.active?.session.sequence, 1);

  current = { ...current, sequence: 2, model: { provider: "openai", model: "gpt-live" } };
  emit(listeners, { method: "session/update", params: { sessionId: "session-1", durableSequence: 2, update: { type: "committed", event: { type: "sessionModelChanged", sessionId: "session-1", model: current.model! } } } });
  await waitFor(() => service.sessions[0]?.sequence === 2);

  assert.deepEqual(service.sessions[0]?.model, { provider: "openai", model: "gpt-live" });
  assert.deepEqual(service.active?.session.model, { provider: "openai", model: "gpt-live" });

  current = { ...current, sequence: 3, status: "archived" };
  emit(listeners, { method: "session/update", params: { sessionId: "session-1", durableSequence: 3, update: { type: "committed", event: { type: "sessionArchived", sessionId: "session-1" } } } });
  await waitFor(() => service.sessions[0]?.status === "archived");

  assert.deepEqual(unsubscribed, ["session-1"]);
  assert.equal(service.active, undefined);
});

test("WorkbenchSessionService stops refreshing when the canonical snapshot cannot reach an announced sequence", async () => {
  const current = session(1);
  const listeners = new Set<(event: ServerNotification) => void>();
  let subscriptions = 0;
  const api: ISessionApi = {
    async create() { return { session: current }; },
    async read() { return { session: current }; },
    async list() { return { sessions: [current] }; },
    async subscribe() { subscriptions += 1; return { session: current, updates: [], threadProjections: [] }; },
    async unsubscribe() {},
    async createThread() { throw new Error("Not used"); },
    async forkThread() { throw new Error("Not used"); },
    async archiveThread() { throw new Error("Not used"); },
    async complete() { throw new Error("Not used"); },
    async archive() { throw new Error("Not used"); },
    async stop() { throw new Error("Not used"); },
    async setModel() { throw new Error("Not used"); },
  };
  const events: IServerEventApi = {
    subscribe(listener) {
      listeners.add(listener);
      return { dispose: () => { listeners.delete(listener); } };
    },
  };
  using service = new WorkbenchSessionService({ session: api, events });
  await service.initialize();

  emit(listeners, { method: "session/update", params: { sessionId: "session-1", durableSequence: 2, update: { type: "committed", event: { type: "sessionModelChanged", sessionId: "session-1", model: { provider: "openai", model: "gpt-live" } } } } });
  await waitFor(() => service.state === "error");

  assert.equal(subscriptions, 2);
  assert.match(service.error ?? "", /did not advance/);
});

function session(sequence: number): SessionDto {
  return {
    sessionId: "session-1",
    title: "Session 1",
    status: "active",
    sequence,
    threads: [{ threadId: "thread-1", origin: { type: "root" }, status: "active" }],
  };
}

function emit(listeners: ReadonlySet<(event: ServerNotification) => void>, event: ServerNotification): void {
  for (const listener of listeners) listener(event);
}

async function waitFor(predicate: () => boolean): Promise<void> {
  for (let attempt = 0; attempt < 30; attempt += 1) {
    if (predicate()) return;
    await new Promise(resolve => setTimeout(resolve, 0));
  }
  assert.fail("Timed out waiting for Session refresh");
}
