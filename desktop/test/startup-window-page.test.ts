import assert from "node:assert/strict";
import test from "node:test";
import {
  createStartupWindowDocument,
  createStartupWindowUrl,
} from "../src/zeta/code/electron-main/startupWindowPage.js";

test("startup window document is inert and escapes external text", () => {
  const document = createStartupWindowDocument(
    "Zeta <Code>",
    {
      kind: "failed",
      message: '<img src=x onerror="alert(1)">',
    },
  );

  assert.match(
    document,
    /Content-Security-Policy[\s\S]*default-src 'none'; style-src 'unsafe-inline';/,
  );
  assert.ok(!document.includes("<script"));
  assert.ok(!document.includes("<img src=x"));
  assert.match(document, /Zeta &lt;Code&gt;/);
  assert.match(
    document,
    /&lt;img src=x onerror=&quot;alert\(1\)&quot;&gt;/,
  );
  assert.match(document, /Startup could not continue/);
});

test("startup window URL contains the encoded starting document", () => {
  const url = createStartupWindowUrl("Zeta Code", {
    kind: "starting",
    message: "Validating App Server",
  });

  assert.ok(url.startsWith("data:text/html;charset=utf-8,"));
  const document = decodeURIComponent(url.slice(url.indexOf(",") + 1));
  assert.match(document, /<div class="spinner"/);
  assert.match(document, /Validating App Server/);
});
