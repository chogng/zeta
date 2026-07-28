import assert from "node:assert/strict";
import test from "node:test";
import {
  createDisconnectedRendererApi,
  WebAppServerUnavailableError,
} from "../src/zeta/platform/app-server/browser/rendererApi.js";

test("disconnected Web renderer API reports stopped without events", async () => {
  const api = createDisconnectedRendererApi();
  const states: string[] = [];
  const subscription = api.appServer.onConnectionState((state) => {
    states.push(state);
  });

  assert.equal(await api.appServer.getConnectionState(), "stopped");
  assert.deepEqual(states, []);
  subscription.dispose();
});

test("disconnected Web renderer API rejects product operations explicitly", async () => {
  const api = createDisconnectedRendererApi();

  await assert.rejects(
    api.session.list(),
    (error: unknown) => {
      assert.ok(error instanceof WebAppServerUnavailableError);
      assert.equal(error.operation, "session.list");
      assert.match(error.message, /no Web host API was provided/);
      return true;
    },
  );
  await assert.rejects(
    api.typst.compile({
      source: "Hello",
    }),
    (error: unknown) => {
      assert.ok(error instanceof WebAppServerUnavailableError);
      assert.equal(error.operation, "typst.compile");
      return true;
    },
  );
});
