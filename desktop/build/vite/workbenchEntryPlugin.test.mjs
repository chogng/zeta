import assert from "node:assert/strict";
import test from "node:test";

import { workbenchEntryPlugin } from "./workbenchEntryPlugin.mjs";

test("Workbench entry redirects root requests to the selected product", () => {
  for (const [rendererEntry, expectedLocation] of [
    ["workbench-code", "/browser/workbench/workbench-code.html"],
    ["workbench-academic", "/browser/workbench/workbench-academic.html"],
  ]) {
    const middleware = configuredMiddleware(rendererEntry);
    const result = invoke(middleware, { method: "GET", url: "/?theme=dark" });
    assert.deepEqual(result, {
      ended: true,
      headers: {
        "Cache-Control": "no-store",
        Location: expectedLocation,
      },
      nextCalled: false,
      statusCode: 302,
    });
  }
});

test("Workbench entry leaves non-root and mutating requests to Vite", () => {
  const middleware = configuredMiddleware("workbench-code");
  for (const request of [
    { method: "GET", url: "/browser/workbench/workbench-code.html" },
    { method: "POST", url: "/" },
  ]) {
    assert.deepEqual(invoke(middleware, request), {
      ended: false,
      headers: {},
      nextCalled: true,
      statusCode: undefined,
    });
  }
});

function configuredMiddleware(rendererEntry) {
  let middleware;
  workbenchEntryPlugin(rendererEntry).configureServer({
    middlewares: {
      use(candidate) {
        middleware = candidate;
      },
    },
  });
  assert.equal(typeof middleware, "function");
  return middleware;
}

function invoke(middleware, request) {
  const headers = {};
  let ended = false;
  let nextCalled = false;
  const response = {
    setHeader(name, value) {
      headers[name] = value;
    },
    end() {
      ended = true;
    },
  };
  middleware(request, response, () => {
    nextCalled = true;
  });
  return { ended, headers, nextCalled, statusCode: response.statusCode };
}
