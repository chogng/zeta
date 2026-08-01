import assert from "node:assert/strict";
import test from "node:test";
import { JsonlFrameDecoder, isAllowedDevOrigin } from "./web-app-server-vite-plugin.mjs";

test("accepts only same-origin loopback WebSocket clients", () => {
  assert.equal(isAllowedDevOrigin("http://127.0.0.1:5173", "127.0.0.1:5173"), true);
  assert.equal(isAllowedDevOrigin("http://localhost:5173", "localhost:5173"), true);
  assert.equal(isAllowedDevOrigin("http://127.0.0.1:5174", "127.0.0.1:5173"), false);
  assert.equal(isAllowedDevOrigin("https://example.com", "example.com"), false);
  assert.equal(isAllowedDevOrigin(undefined, "127.0.0.1:5173"), false);
});

test("decodes bounded UTF-8 JSONL frames across chunks", () => {
  const frames = [];
  const errors = [];
  const decoder = new JsonlFrameDecoder((frame) => frames.push(frame), (error) => errors.push(error));
  const bytes = Buffer.from('{"message":"你好"}\n{"ok":true}\n', "utf8");
  decoder.accept(bytes.subarray(0, 15));
  decoder.accept(bytes.subarray(15));
  assert.deepEqual(frames, ['{"message":"你好"}', '{"ok":true}']);
  assert.deepEqual(errors, []);
});

test("rejects CRLF and oversized JSONL frames", () => {
  const errors = [];
  const crlf = new JsonlFrameDecoder(() => assert.fail("CRLF frame must not be emitted"), (error) => errors.push(error));
  crlf.accept(Buffer.from("{}\r\n"));
  const oversized = new JsonlFrameDecoder(() => assert.fail("oversized frame must not be emitted"), (error) => errors.push(error), 3);
  oversized.accept(Buffer.from("1234"));
  assert.match(errors[0].message, /LF, not CRLF/);
  assert.match(errors[1].message, /exceeds 3 bytes/);
});
