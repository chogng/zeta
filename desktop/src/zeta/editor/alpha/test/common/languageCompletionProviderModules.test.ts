import { strict as assert } from "node:assert";
import test from "node:test";
import { LanguageCompletionProviderRegistry, createLanguageCompletionInvokeContext, type LanguageCompletionProvider } from "../../common/languages/completion/languageCompletionProviders.js";
import { LanguageCompletionProviderModuleHost, LanguageCompletionProviderModuleRegistry, LanguageCompletionProviderModuleState, normalizeLanguageCompletionProviderModuleCatalog } from "../../common/languages/completion/languageCompletionProviderModules.js";

test("Provider module activation registers and removes one atomic provider batch", async () => {
  using providers = new LanguageCompletionProviderRegistry();
  using modules = new LanguageCompletionProviderModuleRegistry();
  using registration = modules.register({
    id: "alpha.typescript",
    load: () => [provider("alpha.member"), provider("alpha.keyword")],
  });
  using host = new LanguageCompletionProviderModuleHost(modules, providers);
  const revisions: number[] = [];
  using listener = providers.onDidChangeProviderCatalog(catalog => revisions.push(catalog.revision));

  assert.deepEqual(await host.setActivation("alpha.typescript", LanguageCompletionProviderModuleState.Active), {
    moduleId: "alpha.typescript",
    state: LanguageCompletionProviderModuleState.Active,
    changed: true,
  });
  assert.deepEqual(providers.providerCatalog.providers.map(entry => entry.id), ["alpha.member", "alpha.keyword"]);
  assert.deepEqual(revisions, [1]);
  assert.equal((await host.setActivation("alpha.typescript", LanguageCompletionProviderModuleState.Active)).changed, false);

  assert.equal((await host.setActivation("alpha.typescript", LanguageCompletionProviderModuleState.Inactive)).changed, true);
  assert.deepEqual(providers.providerCatalog.providers, []);
  assert.deepEqual(revisions, [1, 2]);
});

test("Concurrent activation serializes one module load", async () => {
  using providers = new LanguageCompletionProviderRegistry();
  using modules = new LanguageCompletionProviderModuleRegistry();
  let loads = 0;
  using registration = modules.register({
    id: "language.word",
    load: async () => {
      loads += 1;
      await new Promise<void>(resolve => setImmediate(resolve));
      return [provider("language.word")];
    },
  });
  using host = new LanguageCompletionProviderModuleHost(modules, providers);

  const [first, second] = await Promise.all([
    host.setActivation("language.word", LanguageCompletionProviderModuleState.Active),
    host.setActivation("language.word", LanguageCompletionProviderModuleState.Active),
  ]);

  assert.equal(loads, 1);
  assert.equal(first.changed, true);
  assert.equal(second.changed, false);
});

test("Failed module batches and modules removed during load leave no providers", async () => {
  using providers = new LanguageCompletionProviderRegistry();
  using collision = providers.register(provider("alpha.existing"));
  using modules = new LanguageCompletionProviderModuleRegistry();
  using collisionModule = modules.register({
    id: "alpha.collision",
    load: () => [provider("alpha.new"), provider("alpha.existing")],
  });
  let finishLoad: ((value: readonly LanguageCompletionProvider[]) => void) | undefined;
  const removable = modules.register({
    id: "alpha.removable",
    load: () => new Promise(resolve => {
      finishLoad = resolve;
    }),
  });
  using host = new LanguageCompletionProviderModuleHost(modules, providers);

  await assert.rejects(
    host.setActivation("alpha.collision", LanguageCompletionProviderModuleState.Active),
    /already registered/,
  );
  assert.deepEqual(providers.providerCatalog.providers.map(entry => entry.id), ["alpha.existing"]);

  const loading = host.setActivation("alpha.removable", LanguageCompletionProviderModuleState.Active);
  await new Promise<void>(resolve => setImmediate(resolve));
  removable.dispose();
  finishLoad!([provider("alpha.late")]);
  await assert.rejects(loading, /removed while loading/);
  assert.deepEqual(providers.providerCatalog.providers.map(entry => entry.id), ["alpha.existing"]);
});

test("Removing a module releases its active providers", async () => {
  using providers = new LanguageCompletionProviderRegistry();
  using modules = new LanguageCompletionProviderModuleRegistry();
  const registration = modules.register({
    id: "language.word",
    load: () => [provider("language.word")],
  });
  using host = new LanguageCompletionProviderModuleHost(modules, providers);
  await host.setActivation("language.word", LanguageCompletionProviderModuleState.Active);

  registration.dispose();

  assert.deepEqual(providers.getProviders("plaintext", createLanguageCompletionInvokeContext()), []);
});

test("Disposing a module registry releases active providers through its final catalog", async () => {
  using providers = new LanguageCompletionProviderRegistry();
  const modules = new LanguageCompletionProviderModuleRegistry();
  using registration = modules.register({
    id: "language.word",
    load: () => [provider("language.word")],
  });
  using host = new LanguageCompletionProviderModuleHost(modules, providers);
  await host.setActivation("language.word", LanguageCompletionProviderModuleState.Active);

  modules.dispose();

  assert.deepEqual(providers.providerCatalog.providers, []);
});

test("Provider module catalogs are immutable, revisioned, and unambiguous", () => {
  using modules = new LanguageCompletionProviderModuleRegistry();
  const revisions: number[] = [];
  using listener = modules.onDidChangeModuleCatalog(catalog => revisions.push(catalog.revision));
  using first = modules.register({ id: "language.word", load: () => [provider("language.word")] });
  const second = modules.register({ id: "alpha.typescript", load: () => [provider("alpha.typescript")] });
  second.dispose();

  assert.deepEqual(revisions, [1, 2, 3]);
  assert.deepEqual(modules.moduleCatalog.modules.map(module => module.id), ["language.word"]);
  assert.equal(Object.isFrozen(modules.moduleCatalog), true);
  assert.throws(() => normalizeLanguageCompletionProviderModuleCatalog({
    revision: 1,
    modules: [{ id: "same" }, { id: "same" }],
  }), /Duplicate/);
});

function provider(id: string): LanguageCompletionProvider {
  return {
    id,
    languageIds: ["*"],
    provideCompletions: () => undefined,
  };
}
