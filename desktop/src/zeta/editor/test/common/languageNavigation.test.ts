import assert from "node:assert/strict";
import test from "node:test";
import { URI } from "../../../base/common/uri.js";
import { TextModel } from "../../common/model/textModel.js";
import { TextPosition, TextRange } from "../../common/core/text.js";
import { LanguageFeaturesService } from "../../common/services/languageService.js";

test("language navigation collects provider results with source resource identity and removes duplicates", async () => {
  using languages = new LanguageFeaturesService();
  using model = new TextModel("const answer = value;");
  const source = URI.file("C:\\project\\main.ts");
  const target = URI.file("C:\\project\\value.ts");
  const range = TextRange.from(TextPosition.at(2, 0), TextPosition.at(2, 12));
  const selectionRange = TextRange.from(TextPosition.at(2, 6), TextPosition.at(2, 11));
  let observedResource: URI | undefined;
  languages.registerDefinitionProvider({
    languageIds: ["typescript"],
    provideDefinition: request => {
      observedResource = request.resource;
      return [{ resource: target, range, selectionRange }];
    },
  });
  languages.registerDefinitionProvider({
    languageIds: ["typescript"],
    provideDefinition: () => [{ resource: target, range, selectionRange }],
  });
  using navigation = languages.createLanguageNavigationService(model, source);

  const locations = await navigation.provideDefinition("typescript", TextPosition.at(0, 15));

  assert.equal(observedResource, source);
  assert.deepEqual(locations, [{ resource: target, range, selectionRange }]);
  assert.equal(Object.isFrozen(locations), true);
  assert.equal(Object.isFrozen(locations[0]), true);
});

test("language navigation exposes declaration, implementation, type definition, and references independently", async () => {
  using languages = new LanguageFeaturesService();
  using model = new TextModel("value");
  const source = URI.file("C:\\project\\main.ts");
  const location = { resource: source, range: TextRange.from(TextPosition.at(0, 0), TextPosition.at(0, 5)) };
  let includeDeclaration: boolean | undefined;
  languages.registerDeclarationProvider({ languageIds: ["typescript"], provideDeclaration: () => [location] });
  languages.registerImplementationProvider({ languageIds: ["typescript"], provideImplementation: () => [location] });
  languages.registerTypeDefinitionProvider({ languageIds: ["typescript"], provideTypeDefinition: () => [location] });
  languages.registerReferenceProvider({ languageIds: ["typescript"], provideReferences: request => {
    includeDeclaration = request.includeDeclaration;
    return [location];
  } });
  using navigation = languages.createLanguageNavigationService(model, source);
  const position = TextPosition.at(0, 2);

  assert.equal((await navigation.provideDeclaration("typescript", position)).length, 1);
  assert.equal((await navigation.provideImplementation("typescript", position)).length, 1);
  assert.equal((await navigation.provideTypeDefinition("typescript", position)).length, 1);
  assert.equal((await navigation.provideReferences("typescript", position, false)).length, 1);
  assert.equal(includeDeclaration, false);
});

test("language navigation discards results after the source model changes", async () => {
  using languages = new LanguageFeaturesService();
  using model = new TextModel("value");
  const source = URI.file("C:\\project\\main.ts");
  const pending = deferred<readonly { readonly resource: URI; readonly range: TextRange }[]>();
  languages.registerDefinitionProvider({ languageIds: ["typescript"], provideDefinition: () => pending.promise });
  using navigation = languages.createLanguageNavigationService(model, source);
  const result = navigation.provideDefinition("typescript", TextPosition.at(0, 2));
  model.applyEdits([{ range: TextRange.emptyAt(TextPosition.at(0, 5)), text: "!" }]);
  pending.resolve([{ resource: source, range: TextRange.emptyAt(TextPosition.at(0, 0)) }]);

  assert.deepEqual(await result, []);
});

function deferred<T>(): { readonly promise: Promise<T>; readonly resolve: (value: T) => void } {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>(accept => {
    resolve = accept;
  });
  return { promise, resolve };
}
