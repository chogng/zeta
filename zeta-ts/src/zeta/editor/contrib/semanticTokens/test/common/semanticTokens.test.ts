import assert from "node:assert/strict";
import test from "node:test";
import { TextPosition, TextRange } from "../../../../common/core/text.js";
import { LanguageRequestCancellationReason, LanguageRequestStatus } from "../../../../common/languages/languageRequestCoordinator.js";
import { LanguageFeatureProviderRegistry } from "../../../../common/languageFeatureRegistry.js";
import { TextModel } from "../../../../common/model/textModel.js";
import { type LanguageTokenResult } from "../../../../common/tokens/languageTokens.js";
import { SemanticTokensService, type LanguageSemanticTokensProvider } from "../../common/semanticTokens.js";

test("semantic-token service publishes only current model results", async () => {
	using model = new TextModel("value");
	using providers = new LanguageFeatureProviderRegistry<LanguageSemanticTokensProvider>();
	let resolveResult: ((value: LanguageTokenResult | undefined) => void) | undefined;
	using registration = providers.register({
		languageIds: ["typescript"],
		provideSemanticTokens: () => new Promise<LanguageTokenResult | undefined>(resolve => { resolveResult = resolve; }),
	});
	using service = new SemanticTokensService(model, providers);
	const pending = service.requestTokens("typescript");
	await Promise.resolve();
	model.applyEdits([{ range: TextRange.emptyAt(TextPosition.at(0, 5)), text: "!" }]);
	resolveResult?.({ tokens: [{ range: TextRange.from(TextPosition.at(0, 0), TextPosition.at(0, 5)), tokenType: "variable", modifiers: [] }] });

	assert.deepEqual(await pending, { status: LanguageRequestStatus.Cancelled, requestId: 1, modelVersion: 1, reason: LanguageRequestCancellationReason.ModelChanged });
	assert.equal(service.tokens.result, undefined);
});

test("semantic-token service applies the first matching provider", async () => {
	using model = new TextModel("value");
	using providers = new LanguageFeatureProviderRegistry<LanguageSemanticTokensProvider>();
	using registration = providers.register({
		languageIds: ["typescript"],
		provideSemanticTokens: request => ({ tokens: [{ range: TextRange.from(TextPosition.at(0, 0), TextPosition.at(0, request.snapshot.getText().length)), tokenType: "variable", modifiers: ["readonly"] }] }),
	});
	using service = new SemanticTokensService(model, providers);

	assert.equal((await service.requestTokens("typescript")).status, LanguageRequestStatus.Applied);
	assert.equal(service.tokens.result?.value.tokens[0]?.tokenType, "variable");
	assert.deepEqual(service.tokens.result?.value.tokens[0]?.modifiers, ["readonly"]);
});
