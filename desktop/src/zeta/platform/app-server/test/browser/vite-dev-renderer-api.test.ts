import { strict as assert } from "node:assert";
import test from "node:test";
import { APP_SERVER_SCHEMA_HASH, type ServerNotification } from "../../../../../../generated/app-server/types.js";
import { connectViteDevRendererApi } from "../../../../platform/app-server/browser/webRendererApi.js";
import { WEB_APP_SERVER_CLOSED_EVENT, WEB_APP_SERVER_CONNECTED_EVENT, WEB_APP_SERVER_CONNECT_EVENT, WEB_APP_SERVER_DISCONNECT_EVENT, WEB_APP_SERVER_FRAME_EVENT, WEB_APP_SERVER_PROTOCOL_VERSION, type ViteDevHotContext } from "../../../../platform/app-server/browser/viteDevConnection.js";

class FakeHotContext implements ViteDevHotContext {
  private readonly listeners = new Map<string, Set<(payload: unknown) => void>>();
  readonly requests: Array<Record<string, unknown>> = [];
  readonly sentEvents: string[] = [];

  on(event: string, listener: (payload: unknown) => void): void {
    let listeners = this.listeners.get(event);
    if (!listeners) {
      listeners = new Set();
      this.listeners.set(event, listeners);
    }
    listeners.add(listener);
  }

  off(event: string, listener: (payload: unknown) => void): void {
    this.listeners.get(event)?.delete(listener);
  }

  send(event: string, payload?: unknown): void {
    this.sentEvents.push(event);
    if (event === WEB_APP_SERVER_CONNECT_EVENT) {
      this.emit(WEB_APP_SERVER_CONNECTED_EVENT, {
        protocolVersion: WEB_APP_SERVER_PROTOCOL_VERSION,
        workspaceId: "web-dev:test",
        workspaceRoot: "C:\\workspace",
      });
      return;
    }
    if (event !== WEB_APP_SERVER_FRAME_EVENT || !isRecord(payload) || typeof payload.frame !== "string") return;
    const request = JSON.parse(payload.frame) as Record<string, unknown>;
    this.requests.push(request);
    if (request.method === "initialize") {
      this.respond(request, {
        serverInfo: { name: "zeta-app-server", version: "0.1.0" },
        schemaHash: APP_SERVER_SCHEMA_HASH,
        capabilities: { sessions: true, threads: true, turns: true },
        slashCommands: [],
      });
    } else if (request.method === "session/list") {
      this.respond(request, { sessions: [] });
    }
  }

  emitNotification(notification: ServerNotification): void {
    this.emit(WEB_APP_SERVER_FRAME_EVENT, { frame: JSON.stringify({ jsonrpc: "2.0", ...notification }) });
  }

  close(message: string): void {
    this.emit(WEB_APP_SERVER_CLOSED_EVENT, { message });
  }

  private respond(request: Record<string, unknown>, result: unknown): void {
    this.emit(WEB_APP_SERVER_FRAME_EVENT, { frame: JSON.stringify({ jsonrpc: "2.0", id: request.id, result }) });
  }

  private emit(event: string, payload: unknown): void {
    for (const listener of this.listeners.get(event) ?? []) listener(payload);
  }
}

test("connects, initializes, maps renderer requests, and disposes the Vite bridge", async () => {
  const hot = new FakeHotContext();
  const connected = await connectViteDevRendererApi(hot);
  assert.deepEqual(connected.metadata, { workspaceId: "web-dev:test", workspaceRoot: "C:\\workspace" });
  assert.equal(await connected.api.appServer.getConnectionState(), "ready");
  assert.deepEqual(await connected.api.appServer.getSlashCommands(), []);
  assert.deepEqual(await connected.api.session.list(), { sessions: [] });
  assert.deepEqual(hot.requests.map((request) => request.method), ["initialize", "session/list"]);
  connected.dispose();
  assert.equal(hot.sentEvents.at(-1), WEB_APP_SERVER_DISCONNECT_EVENT);
});

test("delivers App Server notifications and reports bridge closure", async () => {
  const hot = new FakeHotContext();
  const connected = await connectViteDevRendererApi(hot);
  const notifications: ServerNotification[] = [];
  const states: string[] = [];
  connected.api.events.subscribe((notification) => notifications.push(notification));
  connected.api.appServer.onConnectionState((state) => states.push(state));
  const notification: ServerNotification = { method: "fs/changed", params: { type: "pathsChanged", paths: ["README.md"] } };
  hot.emitNotification(notification);
  hot.close("test bridge closed");
  assert.deepEqual(notifications, [notification]);
  assert.deepEqual(states, ["crashed"]);
  assert.equal(await connected.api.appServer.getConnectionState(), "crashed");
  connected.dispose();
});

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
