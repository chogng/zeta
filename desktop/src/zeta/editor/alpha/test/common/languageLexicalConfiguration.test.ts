import { strict as assert } from "node:assert";
import test from "node:test";
import { registerAlphaBuiltinLanguageConfigurations } from "../../common/languageBuiltinConfigurations.js";
import { LanguageConfigurationRegistry } from "../../common/languageConfiguration.js";
import { type LanguageAnalysisProviderRequest } from "../../common/languageAnalysisProviders.js";
import { createLanguageLexicalAnalysisProvider } from "../../common/languageLexicalAnalysisProvider.js";
import { TextModel } from "../../common/textModel.js";

test("Lexical caches remain isolated by language identity at one model version", async () => {
  using model = new TextModel("// comment\n`value`");
  using configurations = new LanguageConfigurationRegistry();
  using builtins = registerAlphaBuiltinLanguageConfigurations(configurations);
  const provider = createLanguageLexicalAnalysisProvider({ languageConfigurations: configurations });
  const snapshot = model.createSnapshot();

  assert.deepEqual(await tokenTypes(provider, request(1, "typescript", snapshot)), ["comment", "string"]);
  assert.deepEqual(await tokenTypes(provider, request(2, "json", snapshot)), ["operator", "variable", "variable"]);
  assert.deepEqual(await tokenTypes(provider, request(3, "jsonc", snapshot)), ["comment", "variable"]);
  assert.deepEqual(await tokenTypes(provider, request(4, "typescript", snapshot)), ["comment", "string"]);
});

test("A language configuration revision replaces same-version lexical state", async () => {
  using model = new TextModel("# comment\n<% value");
  using configurations = new LanguageConfigurationRegistry();
  using builtins = registerAlphaBuiltinLanguageConfigurations(configurations);
  const provider = createLanguageLexicalAnalysisProvider({ languageConfigurations: configurations });
  const snapshot = model.createSnapshot();

  assert.deepEqual(await tokenTypes(provider, request(1, "json", snapshot)), ["variable", "operator", "variable"]);
  using custom = configurations.register("json", {
    comments: { lineComment: "#" },
    brackets: [{ open: "<%", close: "%>" }],
  }, { priority: 10 });

  assert.deepEqual(await tokenTypes(provider, request(2, "json", snapshot)), ["comment", "variable"]);
  const diagnostics = await provider.provideDiagnostics!(request(3, "json", snapshot), new AbortController().signal);
  assert.deepEqual(diagnostics?.diagnostics.map(diagnostic => diagnostic.message), ["Unclosed bracket '<%'"]);
});

test("Built-in lexical configuration registrations release without owning the registry", () => {
  using configurations = new LanguageConfigurationRegistry();
  const registrations = registerAlphaBuiltinLanguageConfigurations(configurations);

  assert.equal(configurations.getLanguageConfiguration("typescript").comments.lineComment, "//");
  assert.equal(configurations.getLanguageConfiguration("json").comments.lineComment, undefined);
  assert.equal(configurations.getLanguageConfiguration("jsonc").comments.lineComment, "//");

  registrations.dispose();
  assert.deepEqual(configurations.getLanguageConfiguration("typescript").comments, {});
  assert.deepEqual(configurations.getLanguageConfiguration("typescript").brackets, []);
});

function request(requestId: number, languageId: string, snapshot: ReturnType<TextModel["createSnapshot"]>): LanguageAnalysisProviderRequest {
  return Object.freeze({ requestId, languageId, snapshot });
}

async function tokenTypes(provider: ReturnType<typeof createLanguageLexicalAnalysisProvider>, analysisRequest: LanguageAnalysisProviderRequest): Promise<readonly string[]> {
  const result = await provider.provideTokens!(analysisRequest, new AbortController().signal);
  return result?.tokens.map(token => token.tokenType) ?? [];
}
