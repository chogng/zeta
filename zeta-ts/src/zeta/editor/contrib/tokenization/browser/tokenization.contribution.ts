import { registerEditorContribution } from "../../../browser/editorContribution.js";
import { TokenizationController } from "./tokenizationController.js";
import { TextEditorCapability } from "../../textEditorCapabilities.js";
import { createAsterSemanticTokenSource, createOverlaySemanticTokenSource } from "../../semanticTokens/browser/semanticTokenPresentation.js";

registerEditorContribution({ id: "editor.contrib.tokenization", configure: context => {
  const lexicalSource = createAsterSemanticTokenSource(context.getCapability(TextEditorCapability.tokenization));
  const semanticOverlay = context.getOptionalCapability(TextEditorCapability.semanticTokenOverlay);
  const source = semanticOverlay ? createOverlaySemanticTokenSource(lexicalSource, createAsterSemanticTokenSource(semanticOverlay)) : lexicalSource;
  context.provideCapability(TextEditorCapability.semanticTokenSource, source);
  context.setSemanticTokenSource(source);
}, install: context => {
  if (context.kind !== "text") return;
  context.own(new TokenizationController(context.viewport, context.getCapability(TextEditorCapability.tokenization)));
} });
