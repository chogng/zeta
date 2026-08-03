import assert from "node:assert/strict";
import test from "node:test";
import { LanguageBracketMatcher } from "../../language/common/languageBracketMatcher.js";
import { LanguageConfigurationRegistry } from "../../language/common/languageConfiguration.js";
import { TextPosition, TextRange } from "../../common/text.js";
import { TextModel } from "../../common/textModel.js";

test("Language bracket matcher resolves nested cross-line configured pairs", () => {
  using model = new TextModel("function value() {\n  return [call(1)];\n}");
  using configurations = bracketConfigurations();
  using matcher = new LanguageBracketMatcher(model, "typescript", configurations);

  assert.deepEqual(matcher.findMatch(TextPosition.at(0, 17)), {
    opening: TextRange.from(TextPosition.at(0, 17), TextPosition.at(0, 18)),
    closing: TextRange.from(TextPosition.at(2, 0), TextPosition.at(2, 1)),
  });
  assert.deepEqual(matcher.findMatch(TextPosition.at(1, 16)), {
    opening: TextRange.from(TextPosition.at(1, 14), TextPosition.at(1, 15)),
    closing: TextRange.from(TextPosition.at(1, 16), TextPosition.at(1, 17)),
  });
});

test("Language bracket matcher ignores strings and comments, and invalidates on edits", () => {
  using model = new TextModel("const text = \"{\"; // [\n{");
  using configurations = bracketConfigurations();
  using matcher = new LanguageBracketMatcher(model, "typescript", configurations);
  assert.equal(matcher.findMatch(TextPosition.at(0, 14)), undefined);
  assert.equal(matcher.findMatch(TextPosition.at(0, 21)), undefined);
  assert.equal(matcher.findMatch(TextPosition.at(1, 0)), undefined);

  model.applyEdits([{
    range: TextRange.emptyAt(TextPosition.at(1, 1)),
    text: "\n}",
  }]);
  assert.deepEqual(matcher.findMatch(TextPosition.at(1, 0)), {
    opening: TextRange.from(TextPosition.at(1, 0), TextPosition.at(1, 1)),
    closing: TextRange.from(TextPosition.at(2, 0), TextPosition.at(2, 1)),
  });
});

test("Language bracket matcher bounds searches and validates construction", () => {
  using model = new TextModel("{\nline\n}");
  using configurations = bracketConfigurations();
  using matcher = new LanguageBracketMatcher(model, "typescript", configurations, { maxScanLineCount: 2 });
  assert.equal(matcher.findMatch(TextPosition.at(0, 0)), undefined);
  assert.throws(() => new LanguageBracketMatcher(model, "typescript", configurations, { maxScanLineCount: 0 }), /positive safe integer/);
});

function bracketConfigurations(): LanguageConfigurationRegistry {
  const configurations = new LanguageConfigurationRegistry();
  configurations.register("typescript", {
    comments: { lineComment: "//", blockComment: { open: "/*", close: "*/" } },
    brackets: [
      { open: "(", close: ")" },
      { open: "[", close: "]" },
      { open: "{", close: "}" },
    ],
  });
  return configurations;
}
