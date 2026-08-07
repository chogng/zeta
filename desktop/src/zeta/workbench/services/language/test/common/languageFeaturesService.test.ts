import { strict as assert } from "node:assert";
import test from "node:test";
import { TextModel } from "../../../../../editor/alpha/common/model/textModel.js";
import { LanguageRequestStatus } from "../../../../../editor/alpha/common/languages/languageRequestCoordinator.js";
import { LanguageFeaturesService } from "../../common/languageFeaturesService.js";

test("Language features service owns shared registrations while document services stay caller-owned", async () => {
  using languageFeatures = new LanguageFeaturesService();
  using model = new TextModel("const answer = 42;");
  using syntax = languageFeatures.createSyntaxService(model);
  using completions = languageFeatures.createCompletionService(model);

  assert.equal(languageFeatures.configurations.getLanguageConfiguration("typescript").comments.lineComment, "//");
  assert.equal((await syntax.requestAll("typescript")).tokens.status, LanguageRequestStatus.Applied);
  assert.equal(completions.textModel, model);
});
