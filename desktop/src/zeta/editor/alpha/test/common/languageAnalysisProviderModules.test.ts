import { strict as assert } from "node:assert";
import test from "node:test";
import { LanguageAnalysisProviderRegistry, type LanguageAnalysisProvider } from "../../common/languages/analysis/languageAnalysisProviders.js";
import { LanguageAnalysisProviderModuleHost, LanguageAnalysisProviderModuleRegistry, LanguageAnalysisProviderModuleState } from "../../common/languages/analysis/languageAnalysisProviderModules.js";

test("Analysis provider module activation installs and removes one provider batch", async () => {
  using providers = new LanguageAnalysisProviderRegistry();
  using modules = new LanguageAnalysisProviderModuleRegistry();
  using registration = modules.register({
    id: "language.lexical",
    load: () => [tokenProvider("alpha.tokens"), diagnosticProvider("alpha.diagnostics")],
  });
  using host = new LanguageAnalysisProviderModuleHost(modules, providers);

  assert.equal((await host.setActivation("language.lexical", LanguageAnalysisProviderModuleState.Active)).changed, true);
  assert.equal(providers.getTokenProvider("typescript")?.id, "alpha.tokens");
  assert.deepEqual(providers.getDiagnosticProviders("typescript").map(provider => provider.id), ["alpha.diagnostics"]);
  assert.equal((await host.setActivation("language.lexical", LanguageAnalysisProviderModuleState.Inactive)).changed, true);
  assert.equal(providers.getTokenProvider("typescript"), undefined);
  assert.deepEqual(providers.getDiagnosticProviders("typescript"), []);
});

test("Failed Analysis provider batches do not leak partial registrations", async () => {
  using providers = new LanguageAnalysisProviderRegistry();
  using existing = providers.register(diagnosticProvider("alpha.existing"));
  using modules = new LanguageAnalysisProviderModuleRegistry();
  using registration = modules.register({
    id: "alpha.collision",
    load: () => [tokenProvider("alpha.transient"), diagnosticProvider("alpha.existing")],
  });
  using host = new LanguageAnalysisProviderModuleHost(modules, providers);

  await assert.rejects(
    host.setActivation("alpha.collision", LanguageAnalysisProviderModuleState.Active),
    /already registered/,
  );
  assert.equal(providers.getTokenProvider("typescript"), undefined);
  assert.deepEqual(providers.getDiagnosticProviders("typescript").map(provider => provider.id), ["alpha.existing"]);
});

function tokenProvider(id: string): LanguageAnalysisProvider {
  return {
    id,
    languageIds: ["*"],
    provideTokens: () => undefined,
  };
}

function diagnosticProvider(id: string): LanguageAnalysisProvider {
  return {
    id,
    languageIds: ["*"],
    provideDiagnostics: () => undefined,
  };
}
