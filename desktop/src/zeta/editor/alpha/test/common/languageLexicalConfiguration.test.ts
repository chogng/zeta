import { strict as assert } from "node:assert";
import test from "node:test";
import { registerBuiltinLanguageConfigurations } from "../../common/languages/languageBuiltinConfigurations.js";
import { LanguageConfigurationRegistry } from "../../common/languages/languageConfiguration.js";
import { type LanguageAnalysisProviderRequest } from "../../common/languages/analysis/languageAnalysisProviders.js";
import { createLanguageLexicalAnalysisProvider } from "../../common/languages/languageLexicalAnalysisProvider.js";
import { TextModel } from "../../common/model/textModel.js";

test("Lexical caches remain isolated by language identity at one model version", async () => {
  using model = new TextModel("// comment\n`value`");
  using configurations = new LanguageConfigurationRegistry();
  using builtins = registerBuiltinLanguageConfigurations(configurations);
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
  using builtins = registerBuiltinLanguageConfigurations(configurations);
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
  const registrations = registerBuiltinLanguageConfigurations(configurations);

  assert.equal(configurations.getLanguageConfiguration("typescript").comments.lineComment, "//");
  assert.equal(configurations.getLanguageConfiguration("json").comments.lineComment, undefined);
  assert.equal(configurations.getLanguageConfiguration("jsonc").comments.lineComment, "//");
  assert.equal(configurations.getLanguageConfiguration("rust").comments.lineComment, "//");
  assert.deepEqual(
    configurations.getLanguageConfiguration("rust").autoClosingPairs.map(pair => pair.open),
    ["(", "[", "{", "\""],
  );

  registrations.dispose();
  assert.deepEqual(configurations.getLanguageConfiguration("typescript").comments, {});
  assert.deepEqual(configurations.getLanguageConfiguration("typescript").brackets, []);
});

test("Rust lexical analysis recognizes Rust comments, keywords, strings, and structural diagnostics", async () => {
  using model = new TextModel("/// docs\nfn main() { let value = \"ok\"; }");
  const provider = createLanguageLexicalAnalysisProvider();
  const snapshot = model.createSnapshot();

  assert.deepEqual(
    await tokenTypes(provider, request(1, "rust", snapshot)),
    ["comment", "keyword", "variable", "keyword", "variable", "operator", "string"],
  );
  const diagnostics = await provider.provideDiagnostics!(request(2, "rust", snapshot), new AbortController().signal);
  assert.deepEqual(diagnostics?.diagnostics, []);
});

test("Rust lexical analysis recognizes hash-delimited raw strings and character literals", async () => {
  using model = new TextModel("let raw = r##\"{\ninside } \"##;\nlet character = '\\n';\nlet lifetime = 'a;");
  const provider = createLanguageLexicalAnalysisProvider();
  const snapshot = model.createSnapshot();

  assert.deepEqual(
    await tokenTypes(provider, request(1, "rust", snapshot)),
    ["keyword", "variable", "operator", "string", "string", "keyword", "variable", "operator", "string", "keyword", "variable", "operator", "variable"],
  );
  const diagnostics = await provider.provideDiagnostics!(request(2, "rust", snapshot), new AbortController().signal);
  assert.deepEqual(diagnostics?.diagnostics, []);
});

test("ECMAScript lexical analysis recognizes regular expressions without mistaking division for a literal", async () => {
  using model = new TextModel("const matcher = /\\{(?<name>[a-z]+)\\}/giu;\nconst ratio = total / count;\nreturn /[{}]/.test(value);");
  const provider = createLanguageLexicalAnalysisProvider();
  const snapshot = model.createSnapshot();

  assert.deepEqual(
    await tokenTypes(provider, request(1, "typescript", snapshot)),
    ["keyword", "variable", "operator", "regexp", "keyword", "variable", "operator", "variable", "operator", "variable", "keyword", "regexp", "variable", "variable"],
  );
  const diagnostics = await provider.provideDiagnostics!(request(2, "typescript", snapshot), new AbortController().signal);
  assert.deepEqual(diagnostics?.diagnostics, []);
});

function request(requestId: number, languageId: string, snapshot: ReturnType<TextModel["createSnapshot"]>): LanguageAnalysisProviderRequest {
  return Object.freeze({ requestId, languageId, snapshot });
}

async function tokenTypes(provider: ReturnType<typeof createLanguageLexicalAnalysisProvider>, analysisRequest: LanguageAnalysisProviderRequest): Promise<readonly string[]> {
  const result = await provider.provideTokens!(analysisRequest, new AbortController().signal);
  return result?.tokens.map(token => token.tokenType) ?? [];
}
