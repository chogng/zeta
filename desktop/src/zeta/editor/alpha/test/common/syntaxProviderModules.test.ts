import { strict as assert } from "node:assert";
import test from "node:test";
import { SyntaxProviderRegistry, type SyntaxProvider } from "../../common/languages/syntax/syntaxProviders.js";
import { SyntaxProviderModuleHost, SyntaxProviderModuleRegistry, SyntaxProviderModuleState } from "../../common/languages/syntax/syntaxProviderModules.js";

test("Syntax provider module activation installs and removes one provider batch", async () => {
  using providers = new SyntaxProviderRegistry();
  using modules = new SyntaxProviderModuleRegistry();
  using registration = modules.register({
    id: "language.lexical",
    load: () => [tokenProvider("alpha.tokens"), diagnosticProvider("alpha.diagnostics")],
  });
  using host = new SyntaxProviderModuleHost(modules, providers);

  assert.equal((await host.setActivation("language.lexical", SyntaxProviderModuleState.Active)).changed, true);
  assert.equal(providers.getTokenProvider("typescript")?.id, "alpha.tokens");
  assert.deepEqual(providers.getDiagnosticProviders("typescript").map(provider => provider.id), ["alpha.diagnostics"]);
  assert.equal((await host.setActivation("language.lexical", SyntaxProviderModuleState.Inactive)).changed, true);
  assert.equal(providers.getTokenProvider("typescript"), undefined);
  assert.deepEqual(providers.getDiagnosticProviders("typescript"), []);
});

test("Failed Syntax provider batches do not leak partial registrations", async () => {
  using providers = new SyntaxProviderRegistry();
  using existing = providers.register(diagnosticProvider("alpha.existing"));
  using modules = new SyntaxProviderModuleRegistry();
  using registration = modules.register({
    id: "alpha.collision",
    load: () => [tokenProvider("alpha.transient"), diagnosticProvider("alpha.existing")],
  });
  using host = new SyntaxProviderModuleHost(modules, providers);

  await assert.rejects(
    host.setActivation("alpha.collision", SyntaxProviderModuleState.Active),
    /already registered/,
  );
  assert.equal(providers.getTokenProvider("typescript"), undefined);
  assert.deepEqual(providers.getDiagnosticProviders("typescript").map(provider => provider.id), ["alpha.existing"]);
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
