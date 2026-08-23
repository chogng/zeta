import { strict as assert } from "node:assert";
import test from "node:test";
import { SyntaxProviderRegistry, type SyntaxProvider } from "../../common/languages/syntax/syntaxProviders.js";
import { SyntaxProviderModuleHost, SyntaxProviderModuleRegistry, SyntaxProviderModuleState } from "../../common/languages/syntax/syntaxProviderModules.js";

test("Syntax provider module activation installs and removes one provider batch", async () => {
  using providers = new SyntaxProviderRegistry();
  using modules = new SyntaxProviderModuleRegistry();
  using registration = modules.register({
    id: "language.lexical",
    load: () => [tokenProvider("aster.tokens"), diagnosticProvider("aster.diagnostics")],
  });
  using host = new SyntaxProviderModuleHost(modules, providers);

  assert.equal((await host.setActivation("language.lexical", SyntaxProviderModuleState.Active)).changed, true);
  assert.equal(providers.getTokenProvider("typescript")?.id, "aster.tokens");
  assert.deepEqual(providers.getDiagnosticProviders("typescript").map(provider => provider.id), ["aster.diagnostics"]);
  assert.equal((await host.setActivation("language.lexical", SyntaxProviderModuleState.Inactive)).changed, true);
  assert.equal(providers.getTokenProvider("typescript"), undefined);
  assert.deepEqual(providers.getDiagnosticProviders("typescript"), []);
});

test("Failed Syntax provider batches do not leak partial registrations", async () => {
  using providers = new SyntaxProviderRegistry();
  using existing = providers.register(diagnosticProvider("aster.existing"));
  using modules = new SyntaxProviderModuleRegistry();
  using registration = modules.register({
    id: "aster.collision",
    load: () => [tokenProvider("aster.transient"), diagnosticProvider("aster.existing")],
  });
  using host = new SyntaxProviderModuleHost(modules, providers);

  await assert.rejects(
    host.setActivation("aster.collision", SyntaxProviderModuleState.Active),
    /already registered/,
  );
  assert.equal(providers.getTokenProvider("typescript"), undefined);
  assert.deepEqual(providers.getDiagnosticProviders("typescript").map(provider => provider.id), ["aster.existing"]);
});

function tokenProvider(id: string): SyntaxProvider {
  return {
    id,
    languageIds: ["*"],
    provideTokens: () => undefined,
  };
}

function diagnosticProvider(id: string): SyntaxProvider {
  return {
    id,
    languageIds: ["*"],
    provideDiagnostics: () => undefined,
  };
}
