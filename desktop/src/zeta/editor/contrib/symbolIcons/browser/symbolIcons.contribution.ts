import { registerEditorContribution } from "../../../browser/editorContribution.js";
import { SymbolIconsController } from "./symbolIconsController.js";
import { TextEditorCapability } from "../../textEditorCapabilities.js";

registerEditorContribution({ id: "editor.contrib.symbolIcons", install: context => {
  if (context.kind !== "text") return;
  const service = context.own(context.languageFeaturesService.createDocumentSymbolService(context.model, { fallbackProviders: context.getOptionalCapability(TextEditorCapability.documentSymbolProviders) ?? [] }));
  context.own(new SymbolIconsController(context.viewport, service, context.languageId, context.onLanguageError));
} });
