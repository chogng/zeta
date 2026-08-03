import { strict as assert } from "node:assert";
import test from "node:test";
import { LanguageConfigurationRegistry } from "../../common/languageConfiguration.js";
import { LanguageLexicalContextIndex } from "../../common/languageLexicalContext.js";
import { TextPosition, TextRange } from "../../common/text.js";
import { TextModel } from "../../common/textModel.js";

test("Lexical context removes only structural brackets inside strings and comments", () => {
  using model = new TextModel("const code = { value: \"}\" }; // {\n/* {\nstill }\n*/ const after = {};");
  using configurations = new LanguageConfigurationRegistry();
  using language = configurations.register("typescript", {
    comments: {
      lineComment: "//",
      blockComment: { open: "/*", close: "*/" },
    },
    brackets: [
      { open: "{", close: "}" },
      { open: "[", close: "]" },
      { open: "(", close: ")" },
    ],
  });
  using context = new LanguageLexicalContextIndex(model, "typescript", configurations);

  assert.equal(context.getStructuralLineContent(0), "const code = { value: \"\" }; // ");
  assert.equal(context.getStructuralLineContent(1), "/* ");
  assert.equal(context.getStructuralLineContent(2), "still ");
  assert.equal(context.getStructuralLineContent(3), "*/ const after = {};");
  const stringStart = model.getLineContent(0).indexOf("\"");
  assert.equal(context.getStructuralLineContent(0, stringStart, stringStart + 3), "\"\"");
  assert.equal(context.getTokenTypeAt(TextPosition.at(0, stringStart + 1)), "string");
  assert.equal(context.getTokenTypeAt(TextPosition.at(0, model.getLineContent(0).length)), "comment");
  assert.equal(context.getTokenTypeAt(TextPosition.at(0, 13)), undefined);
  assert.equal(context.getTokenTypeAt(TextPosition.at(2, 0)), "comment");
});

test("Lexical context invalidates changed suffixes and recomputes multiline state", () => {
  using model = new TextModel("/*\n{\n*/\n{");
  using configurations = new LanguageConfigurationRegistry();
  using language = configurations.register("typescript", {
    comments: { blockComment: { open: "/*", close: "*/" } },
    brackets: [{ open: "{", close: "}" }],
  });
  using context = new LanguageLexicalContextIndex(model, "typescript", configurations);

  assert.equal(context.getStructuralLineContent(1), "");
  assert.equal(context.getStructuralLineContent(3), "{");
  model.applyEdits([{
    range: TextRange.from(TextPosition.at(0, 0), TextPosition.at(0, 2)),
    text: "",
  }]);
  assert.equal(context.getStructuralLineContent(1), "{");
});

test("Lexical context recompiles when the language configuration revision changes", () => {
  using model = new TextModel("\"<\" \"{\"");
  using configurations = new LanguageConfigurationRegistry();
  using base = configurations.register("demo", {
    brackets: [{ open: "{", close: "}" }],
  });
  using context = new LanguageLexicalContextIndex(model, "demo", configurations);
  assert.equal(context.getStructuralLineContent(0), "\"<\" \"\"");

  using override = configurations.register("demo", {
    brackets: [{ open: "<", close: ">" }],
  }, { priority: 1 });
  assert.equal(context.getStructuralLineContent(0), "\"\" \"{\"");
});

test("Lexical context validates slices and disposal without owning borrowed state", () => {
  using model = new TextModel("value");
  using configurations = new LanguageConfigurationRegistry();
  const context = new LanguageLexicalContextIndex(model, "plaintext", configurations);

  assert.throws(() => context.getStructuralLineContent(1), /outside/);
  assert.throws(() => context.getStructuralLineContent(0, 4, 2), /columns/);
  context.dispose();
  assert.throws(() => context.getStructuralLineContent(0), ReferenceError);
  assert.equal(model.getText(), "value");
  assert.equal(configurations.getLanguageConfiguration("plaintext").languageId, "plaintext");
});

test("Lexical context distinguishes closed and unterminated string boundaries", () => {
  using model = new TextModel("\"closed\"\n\"open");
  using configurations = new LanguageConfigurationRegistry();
  using context = new LanguageLexicalContextIndex(model, "typescript", configurations);

  assert.equal(context.getTokenTypeAt(TextPosition.at(0, 4)), "string");
  assert.equal(context.getTokenTypeAt(TextPosition.at(0, 8)), undefined);
  assert.equal(context.getTokenTypeAt(TextPosition.at(1, 5)), "string");
});

test("Lexical context retains multiline identity on empty continuation lines", () => {
  using commentModel = new TextModel("/*\n\n");
  using stringModel = new TextModel("`\n\n");
  using configurations = new LanguageConfigurationRegistry();
  using builtins = configurations.register("typescript", {
    comments: { blockComment: { open: "/*", close: "*/" } },
    brackets: [{ open: "{", close: "}" }],
  });
  using commentContext = new LanguageLexicalContextIndex(commentModel, "typescript", configurations);
  using stringContext = new LanguageLexicalContextIndex(stringModel, "typescript", configurations);

  assert.equal(commentContext.getTokenTypeAt(TextPosition.at(1, 0)), "comment");
  assert.equal(stringContext.getTokenTypeAt(TextPosition.at(1, 0)), "string");
});

test("Lexical context distinguishes line-comment and closed block-comment ends", () => {
  using model = new TextModel("// value\n/* value */");
  using configurations = new LanguageConfigurationRegistry();
  using language = configurations.register("typescript", {
    comments: {
      lineComment: "//",
      blockComment: { open: "/*", close: "*/" },
    },
  });
  using context = new LanguageLexicalContextIndex(model, "typescript", configurations);

  assert.equal(context.getTokenTypeAt(TextPosition.at(0, 8)), "comment");
  assert.equal(context.getTokenTypeAt(TextPosition.at(1, 11)), undefined);
});

test("Rust raw strings do not contribute structural brackets", () => {
  using model = new TextModel("let raw = r#\"{\n} \"#;\n{");
  using configurations = new LanguageConfigurationRegistry();
  using language = configurations.register("rust", {
    brackets: [{ open: "{", close: "}" }],
  });
  using context = new LanguageLexicalContextIndex(model, "rust", configurations);

  assert.equal(context.getStructuralLineContent(0), "let raw = r#\"");
  assert.equal(context.getStructuralLineContent(1), " \"#;");
  assert.equal(context.getStructuralLineContent(2), "{");
  assert.equal(context.getTokenTypeAt(TextPosition.at(1, 1)), "string");
});

test("ECMAScript regular-expression literals do not contribute structural brackets", () => {
  using model = new TextModel("const matcher = /[{]/;\n{");
  using configurations = new LanguageConfigurationRegistry();
  using language = configurations.register("typescript", {
    brackets: [{ open: "{", close: "}" }],
  });
  using context = new LanguageLexicalContextIndex(model, "typescript", configurations);

  assert.equal(context.getStructuralLineContent(0), "const matcher = /[]/;");
  assert.equal(context.getTokenTypeAt(TextPosition.at(0, 17)), "regexp");
  assert.equal(context.getStructuralLineContent(1), "{");
});
