import assert from "node:assert/strict";
import test from "node:test";
import { Position } from "../../../../common/core/position.js";
import { Range } from "../../../../common/core/range.js";
import { LanguageRequestCancellationReason, LanguageRequestStatus } from "../../../../common/languages/languageRequestCoordinator.js";
import { OwnedLanguageFeatureProviderRegistry } from "../../../../common/ownedLanguageFeatureProviderRegistry.js";
import { TextModel } from "../../../../common/model/textModel.js";
import { type LanguageTokenResult } from "../../../../common/tokens/languageTokens.js";
import { SemanticTokensService, type LanguageSemanticTokensProvider } from "../../common/semanticTokens.js";

test("semantic-token service publishes only current model results", async () => {
	using model = new TextModel("value");
	using providers = new OwnedLanguageFeatureProviderRegistry<LanguageSemanticTokensProvider>();
	let resolveResult: ((value: LanguageTokenResult | undefined) => void) | undefined;
	using registration = providers.register({
		languageIds: ["typescript"],
		provideSemanticTokens: () => new Promise<LanguageTokenResult | undefined>(resolve => { resolveResult = resolve; }),
	});
	using service = new SemanticTokensService(model, providers);
	const pending = service.requestTokens("typescript");
	await Promise.resolve();
	model.applyEdits([{ range: Range.fromPositions(new Position((0) + 1, (5) + 1)), text: "!" }]);
	resolveResult?.({ tokens: [{ range: Range.fromPositions(new Position((0) + 1, (0) + 1), new Position((0) + 1, (5) + 1)), tokenType: "variable", modifiers: [] }] });

	assert.deepEqual(await pending, { status: LanguageRequestStatus.Cancelled, requestId: 1, modelVersion: 1, reason: LanguageRequestCancellationReason.ModelChanged });
	assert.equal(service.tokens.result, undefined);
});

test("semantic-token service applies the first matching provider", async () => {
	using model = new TextModel("value");
	using providers = new OwnedLanguageFeatureProviderRegistry<LanguageSemanticTokensProvider>();
	using registration = providers.register({
		languageIds: ["typescript"],
		provideSemanticTokens: request => ({ tokens: [{ range: Range.fromPositions(new Position((0) + 1, (0) + 1), new Position((0) + 1, (request.snapshot.getText().length) + 1)), tokenType: "variable", modifiers: ["readonly"] }] }),
	});
	using service = new SemanticTokensService(model, providers);

	assert.equal((await service.requestTokens("typescript")).status, LanguageRequestStatus.Applied);
	assert.equal(service.tokens.result?.value.tokens[0]?.tokenType, "variable");
	assert.deepEqual(service.tokens.result?.value.tokens[0]?.modifiers, ["readonly"]);
});
