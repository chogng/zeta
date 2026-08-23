import assert from "node:assert/strict";
import test from "node:test";

async function render(pathname = "/docs/architecture") {
  const workerUrl = new URL("../../docs-site/dist/server/index.js", import.meta.url);
  workerUrl.searchParams.set("test", `${process.pid}-${Date.now()}`);
  const { default: worker } = await import(workerUrl.href);

  return worker.fetch(
    new Request(`http://localhost${pathname}`, { headers: { accept: "text/html" } }),
    {
      ASSETS: {
        fetch: async () => new Response("Not found", { status: 404 }),
      },
    },
    {
      waitUntil() {},
      passThroughOnException() {},
    },
  );
}

test("server-renders the documentation index", async () => {
  const response = await render();
  assert.equal(response.status, 200);
  assert.match(response.headers.get("content-type") ?? "", /^text\/html\b/i);

  const html = await response.text();
  assert.match(html, /<title>系统地图 · Zeta 文档<\/title>/i);
  assert.match(html, />系统地图</);
  assert.match(html, /language-mermaid/);
  assert.match(html, /搜索文档/);
  assert.match(html, /本页内容/);
  assert.match(html, /由仓库中的 Markdown 自动生成/);
  assert.match(html, /https:\/\/github\.com\/chogng\/zeta\/edit\/main\/docs\/architecture\.md/);
  assert.match(html, /内容来源/);
  assert.match(html, />docs\/architecture\.md</);
  assert.match(html, /在 GitHub 编辑 docs\/architecture\.md/);
  assert.match(html, /class="source-reference"[^>]+href="https:\/\/github\.com\/chogng\/zeta\/edit\/main\/docs\/architecture\.md"[^>]*>[\s\S]*内容来源[\s\S]*docs\/architecture\.md[\s\S]*↗[\s\S]*<\/a>/);
  assert.doesNotMatch(html, /codex-preview|react-loading-skeleton|Your site is taking shape/i);
});

test("returns not found for an unknown documentation slug", async () => {
  const response = await render("/docs/not-a-real-page");
  assert.equal(response.status, 404);
});
