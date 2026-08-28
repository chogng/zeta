import { registerEditorContribution } from "../../../browser/editorExtensions.js";
import { TokenizationController } from "./tokenizationController.js";
import { TextEditorCapability } from "../../textEditorCapabilities.js";

registerEditorContribution({ id: "editor.contrib.tokenization", configure: context => {
	const styling = context.semanticTokensStylingService;
	const lexicalSource = styling.createSource(context.getCapability(TextEditorCapability.tokenization));
	const semanticOverlay = context.getOptionalCapability(TextEditorCapability.semanticTokenOverlay);
	const source = semanticOverlay ? styling.createOverlay(lexicalSource, styling.createSource(semanticOverlay)) : lexicalSource;
	context.provideCapability(TextEditorCapability.semanticTokenSource, source);
	context.setSemanticTokenSource(source);
}, install: context => {
	if (context.kind !== "text") return;
	context.register(new TokenizationController(context.viewport, context.getCapability(TextEditorCapability.tokenization)));
} });
