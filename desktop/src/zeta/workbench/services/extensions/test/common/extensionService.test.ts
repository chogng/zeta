import { strict as assert } from "node:assert";
import test from "node:test";
import { toDisposable, type IDisposable } from "../../../../../base/common/lifecycle.js";
import type { ExtensionCatalog, ExtensionDescriptor, IExtensionApi } from "../../../../../platform/extensions/common/extensionApi.js";
import { AppServerExtensionService } from "../../browser/appServerExtensionService.js";
import { parseExtensionManifest } from "../../common/extensionService.js";
import { parseJsonc } from "../../common/jsonc.js";
import { parseExtensionSnippetFile } from "../../common/extensionSnippetProvider.js";
import { ExtensionThemeRegistry, parseExtensionTheme } from "../../common/extensionTheme.js";
import type { ITextMateService } from "../../../textMate/common/textMateService.js";
import type { TextMateGrammarDefinition } from "../../../textMate/common/textMateGrammarRegistry.js";

const descriptor: ExtensionDescriptor = Object.freeze({
  id: "zeta.demo",
  name: "demo",
  publisher: "zeta",
  version: "1.0.0",
  displayName: "Demo",
  sourceKind: "builtIn",
  manifestJson: JSON.stringify({
    name: "demo",
    publisher: "zeta",
    version: "1.0.0",
    contributes: {
      grammars: [{ language: "demo", scopeName: "source.demo", path: "./syntaxes/demo.tmLanguage.json" }],
    },
  }),
  manifestSha256: "sha256:test",
});

test("parses TextMate grammar contributions and normalizes package-relative paths", () => {
  const manifest = parseExtensionManifest(JSON.stringify({
    name: "demo",
    publisher: "zeta",
    version: "1.0.0",
    contributes: {
      grammars: [{
        language: "demo",
        scopeName: "source.demo",
        path: "./syntaxes/demo.tmLanguage.json",
        injectTo: ["source.js"],
        tokenTypes: { "comment.demo": "comment" },
      }],
    },
  }), descriptor);

  assert.deepEqual(manifest.contributes.grammars[0], {
    language: "demo",
    scopeName: "source.demo",
    path: "syntaxes/demo.tmLanguage.json",
    injectTo: ["source.js"],
    tokenTypes: { "comment.demo": "comment" },
  });
});

test("fails closed for invalid or escaping grammar contributions", () => {
  assert.throws(() => parseExtensionManifest(JSON.stringify({
    name: "demo",
    publisher: "zeta",
    version: "1.0.0",
    contributes: { grammars: [{ scopeName: "source.demo", path: "../outside.json" }] },
  }), descriptor));
  assert.throws(() => parseExtensionManifest(JSON.stringify({
    name: "demo",
    publisher: "zeta",
    version: "1.0.0",
    contributes: { grammars: [{ scopeName: "source.demo", path: "grammar.json", tokenTypes: { "source.js": "invalid" } }] },
  }), descriptor), /invalid/);
});

test("preserves language, snippet, theme, and advanced TextMate metadata", () => {
  const manifest = parseExtensionManifest(JSON.stringify({
    name: "demo",
    publisher: "zeta",
    version: "1.0.0",
    contributes: {
      languages: [{ id: "demo", extensions: [".demo"], configuration: "./language-configuration.json" }],
      snippets: [{ language: ["demo"], path: "./snippets/demo.json" }],
      themes: [{ label: "Demo Dark", path: "./themes/demo.json", uiTheme: "vs-dark" }],
      grammars: [{
        language: "demo",
        scopeName: "source.demo",
        path: "./syntaxes/demo.tmLanguage.json",
        embeddedLanguages: { "meta.embedded": "javascript" },
        tokenTypes: { "constant.demo": "string" },
        balancedBracketScopes: ["*"],
        unbalancedBracketScopes: ["string.quoted"],
      }],
    },
  }), descriptor);

  assert.deepEqual(manifest.contributes.languages[0], {
    id: "demo",
    aliases: [],
    extensions: [".demo"],
    filenames: [],
    filenamePatterns: [],
    mimetypes: [],
    configuration: "language-configuration.json",
  });
  assert.deepEqual(manifest.contributes.snippets[0], { language: ["demo"], path: "snippets/demo.json" });
  assert.deepEqual(manifest.contributes.themes[0], { label: "Demo Dark", path: "themes/demo.json", uiTheme: "vs-dark" });
  assert.deepEqual(manifest.contributes.grammars[0]!.embeddedLanguages, { "meta.embedded": "javascript" });
  assert.deepEqual(manifest.contributes.grammars[0]!.tokenTypes, { "constant.demo": "string" });
  assert.deepEqual(manifest.contributes.grammars[0]!.balancedBracketScopes, ["*"]);
});

test("parses JSONC snippet files and validates extension theme catalogs", () => {
  const snippets = parseExtensionSnippetFile(parseJsonc(`{
    // comment
    "for": { "prefix": ["for", "loop"], "body": ["for (const item of items) {", "  $0", "}"], },
  }`, "snippet test"), "snippet test");
  assert.deepEqual(snippets[0], {
    name: "for",
    prefixes: ["for", "loop"],
    body: "for (const item of items) {\n  $0\n}",
  });

  const theme = parseExtensionTheme({
    tokenColors: [{ scope: ["comment", "punctuation.definition.comment"], settings: { foreground: "#6A9955", fontStyle: "italic" } }],
    colors: { "editor.foreground": "#D4D4D4" },
  }, "zeta.demo:0", "zeta.demo", "Demo Dark", "vs-dark", "theme test");
  using themes = new ExtensionThemeRegistry();
  themes.replace([theme]);
  assert.equal(themes.currentCatalog.themes[0]!.label, "Demo Dark");
  assert.equal(themes.currentCatalog.themes[0]!.tokenColors.length, 1);
});

test("registers extension grammars transactionally and loads resources through the API", async () => {
  const catalog: ExtensionCatalog = Object.freeze({
    generation: 1,
    extensions: [descriptor],
    diagnostics: [],
  });
  const api: IExtensionApi = {
    list: async () => catalog,
    readResource: async (extensionId, path) => {
      assert.equal(extensionId, "zeta.demo");
      assert.equal(path, "syntaxes/demo.tmLanguage.json");
      return new TextEncoder().encode('{"scopeName":"source.demo","patterns":[]}');
    },
  };
  const definitions: TextMateGrammarDefinition[] = [];
  let disposed = 0;
  const textMateService = {
    grammars: {
      registerGrammar: (definition: TextMateGrammarDefinition): IDisposable => {
        definitions.push(definition);
        return toDisposable(() => {
          disposed += 1;
          const index = definitions.indexOf(definition);
          if (index >= 0) definitions.splice(index, 1);
        });
      },
    },
  } as unknown as ITextMateService;
  const service = new AppServerExtensionService({ api, textMateService });

  await service.start();

  assert.equal(service.currentCatalog, catalog);
  assert.equal(definitions.length, 1);
  assert.equal(await definitions[0]!.loadGrammar(), '{"scopeName":"source.demo","patterns":[]}');
  service.dispose();
  assert.equal(disposed, 1);
});

test("treats an in-flight load cancelled by disposal as normal shutdown", async () => {
  let rejectList: ((error: Error) => void) | undefined;
  const api: IExtensionApi = {
    list: () => new Promise((_resolve, reject) => { rejectList = reject; }),
    readResource: async () => new Uint8Array(),
  };
  const textMateService = { grammars: { registerGrammar: () => toDisposable(() => {}) } } as unknown as ITextMateService;
  const service = new AppServerExtensionService({ api, textMateService });
  const failures: unknown[] = [];
  using listener = service.onDidFail(failure => failures.push(failure.error));

  const starting = service.start();
  service.dispose();
  rejectList?.(new Error("transport disposed"));

  await assert.doesNotReject(starting);
  assert.deepEqual(failures, []);
});
