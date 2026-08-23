import { strict as assert } from "node:assert";
import test from "node:test";
import { TextModel } from "../../../../../editor/common/model/textModel.js";
import { LanguageRequestStatus } from "../../../../../editor/common/languages/languageRequestCoordinator.js";
import { LanguageFeaturesService } from "../../common/languageFeaturesService.js";
import { TextPosition } from "../../../../../editor/common/core/text.js";

test("Language features service owns shared registrations while document services stay caller-owned", async () => {
	using languageFeatures = new LanguageFeaturesService();
	using model = new TextModel("const answer = 42;");
	using syntax = languageFeatures.createSyntaxService(model);
	using completions = languageFeatures.createCompletionService(model);

	assert.equal(languageFeatures.configurations.getLanguageConfiguration("typescript").comments.lineComment, "//");
	assert.equal((await syntax.requestAll("typescript")).tokens.status, LanguageRequestStatus.Applied);
	assert.equal(completions.textModel, model);
});

test("Language features service atomically owns a replaceable cross-kind provider batch", async () => {
	using languageFeatures = new LanguageFeaturesService();
	using model = new TextModel("answer");
	using hover = languageFeatures.createHoverService(model);
	const registration = languageFeatures.registerProviderBatch({ hovers: [{ providerId: "host.first", languageIds: ["typescript"], provideHover: () => ({ contents: ["first"] }) }] });

	assert.deepEqual(await hover.provideHover("typescript", TextPosition.at(0, 1)), { contents: ["first"] });
	registration.replace({ hovers: [{ providerId: "host.second", languageIds: ["typescript"], provideHover: () => ({ contents: ["second"] }) }] });
	assert.deepEqual(await hover.provideHover("typescript", TextPosition.at(0, 1)), { contents: ["second"] });

	registration.dispose();
	assert.equal(await hover.provideHover("typescript", TextPosition.at(0, 1)), undefined);
	assert.throws(() => registration.replace({}), /disposed/);
});
