import assert from "node:assert/strict";
import test from "node:test";
import { parseLaunchConfigurations } from "../../common/launchConfiguration.js";

test("launch configurations parse an explicit generic DAP command", () => {
  const configurations = parseLaunchConfigurations(`{
    // Zeta keeps the adapter launch explicit and language-neutral.
    "version": "0.2.0",
    "configurations": [{
      "name": "Debug app",
      "type": "example",
      "request": "launch",
      "debugAdapter": { "program": "adapter", "args": ["--stdio"] },
      "program": "${"${workspaceFolder}"}/app",
      "stopOnEntry": true,
    }],
  }`);

  assert.deepEqual(configurations, [{
    id: "launch:0:debug-app",
    name: "Debug app",
    type: "example",
    request: "launch",
    adapter: { program: "adapter", arguments: ["--stdio"] },
    arguments: { program: "${workspaceFolder}/app", stopOnEntry: true },
  }]);
});

test("launch configurations reject implicit adapter discovery", () => {
  assert.throws(() => parseLaunchConfigurations('{"version":"0.2.0","configurations":[{"name":"Debug","type":"node","request":"launch"}]}'), /debugAdapter must be an object/);
});
