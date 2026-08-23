import assert from "node:assert/strict";
import test from "node:test";
import { appServerEnvironment, JsonlFrameDecoder, isAllowedDevOrigin } from "./webAppServerPlugin.ts";

test("accepts only same-origin loopback WebSocket clients", () => {
  assert.equal(isAllowedDevOrigin("http://127.0.0.1:5173", "127.0.0.1:5173"), true);
  assert.equal(isAllowedDevOrigin("http://localhost:5173", "localhost:5173"), true);
  assert.equal(isAllowedDevOrigin("http://127.0.0.1:5174", "127.0.0.1:5173"), false);
  assert.equal(isAllowedDevOrigin("https://example.com", "example.com"), false);
  assert.equal(isAllowedDevOrigin(undefined, "127.0.0.1:5173"), false);
});

test("decodes bounded UTF-8 JSONL frames across chunks", () => {
  const frames: string[] = [];
  const errors: Error[] = [];
  const decoder = new JsonlFrameDecoder((frame) => frames.push(frame), (error) => errors.push(error));
  const bytes = Buffer.from('{"message":"你好"}\n{"ok":true}\n', "utf8");
  decoder.accept(bytes.subarray(0, 15));
  decoder.accept(bytes.subarray(15));
  assert.deepEqual(frames, ['{"message":"你好"}', '{"ok":true}']);
  assert.deepEqual(errors, []);
});

test("rejects CRLF and oversized JSONL frames", () => {
  const errors: Error[] = [];
  const crlf = new JsonlFrameDecoder(() => assert.fail("CRLF frame must not be emitted"), (error) => errors.push(error));
  crlf.accept(Buffer.from("{}\r\n"));
  const oversized = new JsonlFrameDecoder(() => assert.fail("oversized frame must not be emitted"), (error) => errors.push(error), 3);
  oversized.accept(Buffer.from("1234"));
  assert.match(errors[0].message, /LF, not CRLF/);
  assert.match(errors[1].message, /exceeds 3 bytes/);
});

test("passes only safe host environment into the development App Server", () => {
  const environment = appServerEnvironment({
    profileRoot: "/profile",
    ripgrep: "/bin/rg",
    workspaceRoot: "/workspace",
    platform: "linux",
    sourceEnvironment: {
      HOME: "/home/zeta",
      LANG: "en_US.UTF-8",
      LC_ALL: "C.UTF-8",
      PATH: "/bin",
      OPENAI_API_KEY: "secret",
      ZETA_PRODUCT_SERVICES_PATH: "/profile/product-services.json",
    },
  });
  assert.deepEqual(environment, {
    HOME: "/home/zeta",
    LANG: "en_US.UTF-8",
    PATH: "/bin",
    LC_ALL: "C.UTF-8",
    ZETA_PROFILE_ROOT: "/profile",
    ZETA_PRODUCT_SERVICES_PATH: "/profile/product-services.json",
    ZETA_RG_PATH: "/bin/rg",
    ZETA_WORKSPACE_ROOT: "/workspace",
  });
});

test("normalizes the Windows host environment without leaking credentials", () => {
  const environment = appServerEnvironment({
    profileRoot: "C:\\profile",
    ripgrep: "C:\\bin\\rg.exe",
    workspaceRoot: "C:\\workspace",
    platform: "win32",
    sourceEnvironment: {
      Path: "C:\\Windows\\System32",
      SystemRoot: "C:\\Windows",
      UserProfile: "C:\\Users\\zeta",
      AWS_SECRET_ACCESS_KEY: "secret",
    },
  });
  assert.equal(environment.PATH, "C:\\Windows\\System32");
  assert.equal(environment.SYSTEMROOT, "C:\\Windows");
  assert.equal(environment.USERPROFILE, "C:\\Users\\zeta");
  assert.equal(environment.AWS_SECRET_ACCESS_KEY, undefined);
});
