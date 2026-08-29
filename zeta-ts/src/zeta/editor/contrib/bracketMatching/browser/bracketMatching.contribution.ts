import { registerEditorContribution } from '../../../browser/editorExtensions.js';
import { DecorationPresentation, createStanzaDecorationSource } from '../../../browser/viewparts/decorations/decorations.js';
import { LanguageBracketPairs } from '../../../common/languages/languageBracketPairs.js';
import { TextDecorationCollection } from '../../../common/model/decorationCollection.js';
import { TextEditorCapability } from '../../textEditorCapabilities.js';
import { BracketColorizationSource } from './bracketColorizationPresentation.js';
import { BracketEditingController, RemoveBracketsCommandId } from './bracketEditingController.js';
import { BracketMatchController } from './bracketMatchController.js';
import { BracketNavigationController } from './bracketNavigationController.js';

registerEditorContribution({
	id: 'editor.contrib.bracketMatching',
	commands: [{ id: RemoveBracketsCommandId, canTriggerInlineEdits: true }],
	configure: context => {
		const lexicalContext = context.getCapability(TextEditorCapability.languageLexicalContext);
		const largeFile = context.model.largeFile.tooLargeForTokenization;
		const bracketPairs = context.register(new LanguageBracketPairs(context.model, lexicalContext));
		const decorations = context.register(new TextDecorationCollection<void>(context.model));
		context.provideCapability(TextEditorCapability.bracketPairs, bracketPairs);
		context.provideCapability(TextEditorCapability.bracketDecorations, decorations);
		context.addDecorationSource(createStanzaDecorationSource(decorations, () => DecorationPresentation.BracketMatch));
		const colorizeBrackets = context.options.bracketPairColorization !== false;
		const renderBracketGuides = context.options.guides?.bracketPairs !== undefined && context.options.guides.bracketPairs !== false;
		if (!largeFile && (colorizeBrackets || renderBracketGuides)) {
			context.setBracketColorizationSource(new BracketColorizationSource(bracketPairs, colorizeBrackets));
		}
	},
	install: context => {
		if (context.kind !== 'text') return;
		const bracketPairs = context.getCapability(TextEditorCapability.bracketPairs);
		context.register(new BracketMatchController(context.selections, bracketPairs, context.getCapability(TextEditorCapability.bracketDecorations), context.options.matchBrackets ?? 'always'));
		context.register(new BracketNavigationController(context.view.element, context.viewport, context.selections, bracketPairs));
		context.register(new BracketEditingController(context.view.element, context.viewport, context.selections, bracketPairs, context.executeCommand));
	},
});
