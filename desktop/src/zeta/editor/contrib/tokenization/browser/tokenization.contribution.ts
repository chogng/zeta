import { registerEditorContribution } from "../../../browser/editorContribution.js";
import { TokenizationController } from "./tokenizationController.js";
import { TextEditorCapability } from "../../textEditorCapabilities.js";
import { createAsterSemanticTokenSource } from "../../semanticTokens/browser/semanticTokenPresentation.js";

registerEditorContribution({ id: "editor.contrib.tokenization", configure: context => {
  const source = createAsterSemanticTokenSource(context.getCapability(TextEditorCapability.tokenization));
  context.provideCapability(TextEditorCapability.semanticTokenSource, source);
  context.setSemanticTokenSource(source);
}, install: context => {
  if (context.kind !== "text") return;
  context.own(new TokenizationController(context.viewport, context.getCapability(TextEditorCapability.tokenization)));
} });
