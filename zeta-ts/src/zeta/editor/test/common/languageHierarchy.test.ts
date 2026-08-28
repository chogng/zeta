import assert from "node:assert/strict";
import test from "node:test";
import { URI } from "../../../base/common/uri.js";
import { TextPosition, TextRange } from "../../common/core/text.js";
import { TextModel } from "../../common/model/textModel.js";
import { LanguageFeaturesService } from "../../common/services/languageFeaturesService.js";
import { LanguageConfigurationService } from '../../common/services/languageConfigurationService.js';
import { LanguageHierarchyService } from '../../contrib/callHierarchy/common/languageHierarchy.js';

test("language hierarchy keeps prepare and follow-up requests on the same provider", async () => {
	using configurations = new LanguageConfigurationService();
	using languages = new LanguageFeaturesService(configurations);
	using model = new TextModel("function root() {}\n");
	const source = URI.file("C:\\project\\main.ts");
	const root = item("root", source, 0);
	const caller = item("caller", URI.file("C:\\project\\caller.ts"), 2);
	let followedData: unknown;
	languages.callHierarchyProvider.register({
		languageIds: ["typescript"],
		prepareCallHierarchy: request => {
			assert.equal(request.resource, source);
			return [root];
		},
		provideIncomingCalls: request => {
			followedData = request.item.data;
			return [{ item: caller, fromResource: caller.resource, fromRanges: [caller.selectionRange] }];
		},
		provideOutgoingCalls: () => [],
	});
	using service = new LanguageHierarchyService(model, source, languages.callHierarchyProvider, languages.typeHierarchyProvider);

	const prepared = await service.prepareCallHierarchy("typescript", TextPosition.at(0, 10));
	const incoming = await prepared[0]!.incoming(prepared[0]!.roots[0]!);

	assert.deepEqual(followedData, { opaque: "root" });
	assert.equal(incoming[0]!.item.name, "caller");
	assert.equal(Object.isFrozen(incoming), true);
});

test("language hierarchy discards follow-up results when the source revision changes", async () => {
	using configurations = new LanguageConfigurationService();
	using languages = new LanguageFeaturesService(configurations);
	using model = new TextModel("class Root {}\n");
	const source = URI.file("C:\\project\\main.ts");
	const root = item("Root", source, 0);
	const pending = deferred<readonly ReturnType<typeof item>[]>();
	languages.typeHierarchyProvider.register({
		languageIds: ["typescript"],
		prepareTypeHierarchy: () => [root],
		provideSupertypes: () => pending.promise,
		provideSubtypes: () => [],
	});
	using service = new LanguageHierarchyService(model, source, languages.callHierarchyProvider, languages.typeHierarchyProvider);
	const prepared = await service.prepareTypeHierarchy("typescript", TextPosition.at(0, 7));
	const result = prepared[0]!.supertypes(root);
	model.applyEdits([{ range: TextRange.emptyAt(TextPosition.at(0, 13)), text: " " }]);
	pending.resolve([item("Base", source, 1)]);
	assert.deepEqual(await result, []);
});

function item(name: string, resource: URI, line: number) {
	const range = TextRange.from(TextPosition.at(line, 0), TextPosition.at(line, name.length + 2));
	const selectionRange = TextRange.from(TextPosition.at(line, 1), TextPosition.at(line, name.length + 1));
	return { name, symbolKind: 12, resource, range, selectionRange, data: { opaque: name } };
}

function deferred<T>(): { readonly promise: Promise<T>; readonly resolve: (value: T) => void } {
	let resolve!: (value: T) => void;
	const promise = new Promise<T>(accept => { resolve = accept; });
	return { promise, resolve };
}
