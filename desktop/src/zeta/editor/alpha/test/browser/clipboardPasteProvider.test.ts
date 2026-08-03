import assert from "node:assert/strict";
import test from "node:test";
import { AlphaUriListPasteProvider, captureAlphaClipboardTextTransfer, normalizeAlphaClipboardPasteProviders, provideAlphaClipboardPaste, type AlphaClipboardPasteProvider, type AlphaClipboardTextTransfer } from "../../browser/clipboardPasteProvider.js";

test("Clipboard paste providers receive only a synchronous textual transfer snapshot", () => {
  const transfer = captureAlphaClipboardTextTransfer({
    types: ["text/uri-list", "application/x-zeta-snippet"],
    getData(type: string): string {
      return type === "text/uri-list" ? "file:///tmp/example.rs" : "snippet";
    },
  } as unknown as DataTransfer);

  assert.deepEqual(transfer.types, ["text/uri-list", "application/x-zeta-snippet"]);
  assert.equal(transfer.getText("text/uri-list"), "file:///tmp/example.rs");
  assert.equal(transfer.getText("missing"), "");
});

test("Clipboard paste providers validate identity and preserve declared precedence", async () => {
  assert.throws(() => normalizeAlphaClipboardPasteProviders([
    provider("duplicate", ["application/x-first"], () => "first"),
    provider("duplicate", ["application/x-second"], () => "second"),
  ]), /Duplicate Alpha clipboard paste provider/);
  assert.throws(() => normalizeAlphaClipboardPasteProviders([
    provider("missing-types", [], () => "ignored"),
  ]), /requires MIME types/);

  const providers = normalizeAlphaClipboardPasteProviders([
    provider("first", ["application/x-zeta-snippet"], () => undefined),
    provider("second", ["application/x-zeta-snippet"], () => "provided"),
  ]);
  const transfer = textTransfer("application/x-zeta-snippet", "source");
  assert.equal(await provideAlphaClipboardPaste(providers, transfer), "provided");
});

test("URI-list paste omits comments and preserves stable URI order", async () => {
  const transfer = textTransfer("text/uri-list", "# copied locations\r\nfile:///workspace/one.rs\n\nhttps://example.test/two\n");
  assert.equal(await provideAlphaClipboardPaste([AlphaUriListPasteProvider], transfer), "file:///workspace/one.rs\nhttps://example.test/two");
});

function provider(id: string, mimeTypes: readonly string[], providePaste: AlphaClipboardPasteProvider["providePaste"]): AlphaClipboardPasteProvider {
  return { id, mimeTypes, providePaste };
}

function textTransfer(type: string, text: string): AlphaClipboardTextTransfer {
  return Object.freeze({
    types: Object.freeze([type]),
    getText: (requestedType: string): string => requestedType === type ? text : "",
  });
}
