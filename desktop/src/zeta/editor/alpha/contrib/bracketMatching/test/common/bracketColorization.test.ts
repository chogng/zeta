import assert from "node:assert/strict";
import test from "node:test";
import { LanguageBracketColorizationIndex } from "../../common/bracketColorization.js";
import { LanguageConfigurationRegistry } from "../../../../common/languages/languageConfiguration.js";
import { LanguageLexicalContextIndex } from "../../../../common/languages/languageLexicalContext.js";
import { TextRange } from "../../../../common/core/text.js";
import { TextModel } from "../../../../common/model/textModel.js";

test("Bracket colorization follows lexical nesting and excludes brackets in strings", () => {
  using model = new TextModel("{\n  (\"}\")\n}");
  using configurations = new LanguageConfigurationRegistry();
  using registration = configurations.register("typescript", {
    brackets: [{ open: "{", close: "}" }, { open: "(", close: ")" }],
  });
  using lexical = new LanguageLexicalContextIndex(model, "typescript", configurations);
  using colors = new LanguageBracketColorizationIndex(model, lexical);

  assert.deepEqual(colors.getLineColorizations(0), [{ startColumn: 0, endColumn: 1, level: 1 }]);
  assert.deepEqual(colors.getLineColorizations(1), [
    { startColumn: 2, endColumn: 3, level: 2 },
    { startColumn: 6, endColumn: 7, level: 2 },
  ]);
  assert.deepEqual(colors.getLineColorizations(2), [{ startColumn: 0, endColumn: 1, level: 1 }]);
});

test("Bracket colorization invalidates its cached nesting after model edits", () => {
  using model = new TextModel("{\n}");
  using configurations = new LanguageConfigurationRegistry();
  using registration = configurations.register("typescript", { brackets: [{ open: "{", close: "}" }] });
  using lexical = new LanguageLexicalContextIndex(model, "typescript", configurations);
  using colors = new LanguageBracketColorizationIndex(model, lexical, 2);
  assert.deepEqual(colors.getLineColorizations(1), [{ startColumn: 0, endColumn: 1, level: 1 }]);
  model.applyEdits([{ range: TextRange.emptyAt(model.positionAt(0)), text: "{\n" }]);
  assert.deepEqual(colors.getLineColorizations(2), [{ startColumn: 0, endColumn: 1, level: 2 }]);
});
