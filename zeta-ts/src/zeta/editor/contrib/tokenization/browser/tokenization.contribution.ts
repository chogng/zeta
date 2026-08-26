import { registerEditorContribution } from "../../../browser/editorExtensions.js";
import { TokenizationController } from "./tokenizationController.js";
import { TextEditorCapability } from "../../textEditorCapabilities.js";
import { createStanzaSemanticTokenSource, createOverlaySemanticTokenSource } from "../../semanticTokens/browser/semanticTokenPresentation.js";

registerEditorContribution({ id: "editor.contrib.tokenization", configure: context => {
	const lexicalSource = createStanzaSemanticTokenSource(context.getCapability(TextEditorCapability.tokenization));
	const semanticOverlay = context.getOptionalCapability(TextEditorCapability.semanticTokenOverlay);
	const source = semanticOverlay ? createOverlaySemanticTokenSource(lexicalSource, createStanzaSemanticTokenSource(semanticOverlay)) : lexicalSource;
	context.provideCapability(TextEditorCapability.semanticTokenSource, source);
	context.setSemanticTokenSource(source);
}, install: context => {
	if (context.kind !== "text") return;
	context.own(new TokenizationController(context.viewport, context.getCapability(TextEditorCapability.tokenization)));
} });
