import assert from "node:assert/strict";
import test from "node:test";
import { TokenizationTextModelPart } from "../../common/tokenizationTextModelPart.js";
import { LanguageTokenLineIndex } from "../../../../common/tokens/languageTokenLineIndex.js";
import { createLanguageTokenStore } from "../../../../common/languages/languageResults.js";
import { TextModel } from "../../../../common/model/textModel.js";

test("Tokenization text model part exposes the versioned token index contract", () => {
	using model = new TextModel("const value = 1;");
	using store = createLanguageTokenStore(model);
	using index = new LanguageTokenLineIndex(store);
	using part = new TokenizationTextModelPart(index);
	assert.equal(part.textModel, model);
	assert.equal(part.tokenCount, 0);
	assert.deepEqual(part.lines, []);
});
