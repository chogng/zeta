import assert from "node:assert/strict";
import { EventEmitter } from "node:events";
import { mkdtemp, mkdir, readFile, rm, unlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { setTimeout as delay } from "node:timers/promises";

import { productIconsPlugin } from "./product-icons-vite-plugin.mjs";
import { checkProductIcons, syncProductIcons } from "./sync-product-icons.mjs";

const addSvg = `<svg width="16" height="16" viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <path stroke="black" d="M2.5 8h11M8 2.5v11"/>
</svg>
`;

test("product icon generation tracks added, changed, and removed SVG files", async () => {
  const root = await mkdtemp(join(tmpdir(), "zeta-product-icons-"));
  const sourceDirectory = join(root, "icons");
  const outputFile = join(root, "generated", "product-icons.ts");
  try {
    await mkdir(sourceDirectory);
    await writeFile(join(sourceDirectory, "add.svg"), addSvg);
    let report = await syncProductIcons({ outputFile, sourceDirectory });
    let generated = await readFile(outputFile, "utf8");
    assert.equal(report.count, 1);
    assert.equal(report.outputChanged, true);
    assert.match(generated, /export \{ iconAdd as add \}/);
    assert.match(generated, /viewBox=\\"0 0 16 16\\"/);
    assert.doesNotMatch(generated, /\bwidth=\\"16\\"/);
    assert.doesNotMatch(generated, /\bheight=\\"16\\"/);
    assert.equal((await syncProductIcons({ outputFile, sourceDirectory })).outputChanged, false);

    await writeFile(join(sourceDirectory, "close.svg"), addSvg.replace("black", "red"));
    report = await syncProductIcons({ outputFile, sourceDirectory });
    generated = await readFile(outputFile, "utf8");
    assert.equal(report.count, 2);
    assert.match(generated, /export \{ iconClose as close \}/);
    assert.match(generated, /stroke=\\"red\\"/);

    await unlink(join(sourceDirectory, "close.svg"));
    report = await syncProductIcons({ outputFile, sourceDirectory });
    generated = await readFile(outputFile, "utf8");
    assert.equal(report.count, 1);
    assert.doesNotMatch(generated, /export \{ iconClose as close \}/);
  } finally {
    await rm(root, { force: true, recursive: true });
  }
});

test("product icon generation exposes prefix-free names including reserved words", async () => {
  const root = await mkdtemp(join(tmpdir(), "zeta-product-icons-"));
  const sourceDirectory = join(root, "icons");
  const outputFile = join(root, "generated", "product-icons.ts");
  try {
    await mkdir(sourceDirectory);
    await writeFile(join(sourceDirectory, "export.svg"), addSvg);
    await syncProductIcons({ outputFile, sourceDirectory });
    const generated = await readFile(outputFile, "utf8");
    assert.match(generated, /const iconExport/);
    assert.match(generated, /export \{ iconExport as export \}/);
  } finally {
    await rm(root, { force: true, recursive: true });
  }
});

test("product icon synchronization canonicalizes sources and supports a read-only check", async () => {
  const root = await mkdtemp(join(tmpdir(), "zeta-product-icons-"));
  const sourceDirectory = join(root, "icons");
  const outputFile = join(root, "generated", "product-icons.ts");
  try {
    await mkdir(sourceDirectory);
    await writeFile(join(sourceDirectory, "add.svg"), addSvg);
    await assert.rejects(checkProductIcons({ sourceDirectory }), /require synchronization/);
    const report = await syncProductIcons({ outputFile, sourceDirectory });
    const optimized = await readFile(join(sourceDirectory, "add.svg"), "utf8");
    assert.equal(report.sourceChanged, true);
    assert.doesNotMatch(optimized, /\bwidth="16"/);
    assert.doesNotMatch(optimized, /\bheight="16"/);
    assert.match(optimized, /viewBox="0 0 16 16"/);
    assert.equal((await checkProductIcons({ sourceDirectory })).count, 1);
  } finally {
    await rm(root, { force: true, recursive: true });
  }
});

test("product icon generation rejects linked or active SVG content", async () => {
  const root = await mkdtemp(join(tmpdir(), "zeta-product-icons-"));
  const sourceDirectory = join(root, "icons");
  const outputFile = join(root, "generated", "product-icons.ts");
  try {
    await mkdir(sourceDirectory);
    await writeFile(join(sourceDirectory, "linked.svg"), `<svg viewBox="0 0 16 16"><a href="https://example.test"><path d="M0 0h1v1z"/></a></svg>`);
    await assert.rejects(syncProductIcons({ outputFile, sourceDirectory }), /unsupported active or external content/);
  } finally {
    await rm(root, { force: true, recursive: true });
  }
});

test("Vite product icon integration regenerates and reloads after an SVG replacement", async () => {
  const root = await mkdtemp(join(tmpdir(), "zeta-product-icons-vite-"));
  const sourceDirectory = join(root, "icons");
  const outputFile = join(root, "generated", "product-icons.ts");
  try {
    await mkdir(sourceDirectory);
    const sourceFile = join(sourceDirectory, "close.svg");
    await writeFile(sourceFile, '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><path d="M3 3l10 10"/></svg>\n');

    const watcher = new EventEmitter();
    watcher.add = () => undefined;
    const reload = Promise.withResolvers();
    const reloadMessages = [];
    const plugin = productIconsPlugin({ sourceDirectory, outputFile, debounceMilliseconds: 0 });
    plugin.configureServer({
      watcher,
      ws: {
        send(message) {
          reloadMessages.push(message);
          reload.resolve(message);
        },
      },
      config: {
        logger: {
          error(message) {
            reload.reject(new Error(message));
          },
        },
      },
    });

    await writeFile(sourceFile, '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><path d="M1 1l14 14"/></svg>\n');
    watcher.emit("all", "change", sourceFile);
    assert.deepEqual(await reload.promise, { type: "full-reload" });
    assert.match(await readFile(outputFile, "utf8"), /d=\\"m1 1 14 14\\"/);
    assert.equal(await readFile(sourceFile, "utf8"), '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><path d="m1 1 14 14"/></svg>\n');

    watcher.emit("all", "change", sourceFile);
    await delay(25);
    assert.equal(reloadMessages.length, 1);
  } finally {
    await rm(root, { force: true, recursive: true });
  }
});
