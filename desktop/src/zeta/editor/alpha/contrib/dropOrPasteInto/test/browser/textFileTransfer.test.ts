import assert from "node:assert/strict";
import test from "node:test";
import { ALPHA_TEXT_FILE_TRANSFER_MAX_BYTES, selectAlphaTextFileTransfer, type AlphaTextFileTransfer } from "../../browser/textFileTransfer.js";

function file(name: string, type: string, size = 1): AlphaTextFileTransfer {
  return {
    name,
    type,
    size,
    text: async () => "text",
  };
}

test("Text file transfer accepts one bounded textual browser file", () => {
  const rust = file("snippet.rs", "");
  const plain = file("untitled", "text/plain");
  assert.equal(selectAlphaTextFileTransfer([rust]), rust);
  assert.equal(selectAlphaTextFileTransfer([plain]), plain);
});

test("Text file transfer leaves binary, unknown, multi-file, and oversized transfers to the host", () => {
  assert.equal(selectAlphaTextFileTransfer([file("image.png", "image/png")]), undefined);
  assert.equal(selectAlphaTextFileTransfer([file("unknown", "")]), undefined);
  assert.equal(selectAlphaTextFileTransfer([file("a.txt", "text/plain"), file("b.txt", "text/plain")]), undefined);
  assert.equal(selectAlphaTextFileTransfer([file("large.txt", "text/plain", ALPHA_TEXT_FILE_TRANSFER_MAX_BYTES + 1)]), undefined);
});
