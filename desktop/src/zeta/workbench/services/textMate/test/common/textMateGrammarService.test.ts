import assert from "node:assert/strict";
import test from "node:test";
import { TextMateGrammarService } from "../../common/textMateGrammarService.js";

test("grammar service materializes registered contributions into its catalog", async () => {
  using service = new TextMateGrammarService();
  using registration = service.registerGrammar({
    scopeName: "source.alpha",
    languageId: "alpha",
    loadGrammar: () => grammar("source.alpha"),
  });

  const catalog = await service.whenReady();
  assert.equal(catalog.revision, 1);
  assert.deepEqual(catalog.grammars.map(entry => entry.scopeName), ["source.alpha"]);
  assert.equal(service.currentCatalog, catalog);
});

test("grammar service publishes only the newest complete revision", async () => {
  using service = new TextMateGrammarService();
  let resolveGrammar: ((value: string) => void) | undefined;
  const delayedGrammar = new Promise<string>(resolve => {
    resolveGrammar = resolve;
  });
  const revisions: number[] = [];
  using listener = service.onDidChangeCatalog(catalog => revisions.push(catalog.revision));
  using first = service.registerGrammar({
    scopeName: "source.alpha",
    languageId: "alpha",
    loadGrammar: () => delayedGrammar,
  });
  using second = service.registerGrammar({
    scopeName: "source.beta",
    languageId: "beta",
    loadGrammar: () => grammar("source.beta"),
  });

  resolveGrammar?.(grammar("source.alpha"));
  const catalog = await service.whenReady();
  assert.equal(catalog.revision, 2);
  assert.deepEqual(catalog.grammars.map(entry => entry.languageId), ["alpha", "beta"]);
  assert.deepEqual(revisions, [2]);
});

test("grammar service preserves the last good catalog when a loader fails", async () => {
  using service = new TextMateGrammarService();
  using first = service.registerGrammar({
    scopeName: "source.alpha",
    languageId: "alpha",
    loadGrammar: () => grammar("source.alpha"),
  });
  const previous = await service.whenReady();
  const failures: { revision: number; error: unknown }[] = [];
  using listener = service.onDidFailCatalog(failure => failures.push(failure));
  using broken = service.registerGrammar({
    scopeName: "source.broken",
    languageId: "broken",
    loadGrammar: () => {
      throw new Error("broken grammar");
    },
  });

  await assert.rejects(service.whenReady(), /broken grammar/);
  assert.equal(service.currentCatalog, previous);
  assert.equal(failures.length, 1);
  assert.equal(failures[0]?.revision, 2);
});

test("disposing a registration republishes the remaining catalog", async () => {
  using service = new TextMateGrammarService();
  const first = service.registerGrammar({
    scopeName: "source.alpha",
    languageId: "alpha",
    loadGrammar: () => grammar("source.alpha"),
  });
  using second = service.registerGrammar({
    scopeName: "source.beta",
    languageId: "beta",
    loadGrammar: () => grammar("source.beta"),
  });
  await service.whenReady();

  first.dispose();
  const catalog = await service.whenReady();
  assert.equal(catalog.revision, 3);
  assert.deepEqual(catalog.grammars.map(entry => entry.languageId), ["beta"]);
});

function grammar(scopeName: string): string {
  return JSON.stringify({ scopeName, patterns: [] });
}
