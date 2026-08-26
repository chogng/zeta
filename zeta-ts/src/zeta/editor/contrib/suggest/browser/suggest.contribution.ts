import { registerEditorContribution, type EditorCapability } from "../../../browser/editorExtensions.js";
import { type EditorResourceInput } from "../../../common/editorResource.js";
import { type LanguageCompletionService } from "../../../common/languages/completion/languageCompletionService.js";
import { LanguageCompletionSessionController } from "../common/suggestModel.js";
import { SuggestController } from "./suggestController.js";

interface SuggestContributionState {
	readonly service: LanguageCompletionService;
	readonly session: LanguageCompletionSessionController;
}

const suggestState: EditorCapability<SuggestContributionState> = {
	id: "editor.suggest.state",
};

registerEditorContribution({
	id: "editor.contrib.suggest",
	configure: context => {
		if (context.options.suggestions === false) return;
		const completions = context.own(context.languageFeaturesService.createCompletionService(context.model, {
			resource: context.options.input.resource,
			...(context.options.completionWorkerFactory ? { workerFactory: context.options.completionWorkerFactory } : {}),
		}));
		const session = context.own(new LanguageCompletionSessionController(completions.results, context.selections, {
			resolver: completions,
			onResolveError: context.onLanguageError,
			onDidAccept: item => completions.executeCompletionCommand(context.languageId, item, new AbortController().signal),
			snippetVariables: createSnippetVariables(context.options.input),
		}));
		context.provideCapability(suggestState, { service: completions, session });
	},
	install: context => {
		if (context.kind !== "text") return;
		const state = context.getOptionalCapability(suggestState);
		if (!state) return;
		context.own(new SuggestController(
			context.view,
			context.selections,
			state.service,
			state.session,
			context.languageId,
			{ onRequestError: context.onLanguageError },
		));
	},
});

function createSnippetVariables(input: EditorResourceInput): { readonly resolveVariable: (name: string) => string | undefined } {
	const filePath = decodeURIComponent(input.resource.path);
	const separator = filePath.lastIndexOf("/");
	const filename = filePath.slice(separator + 1);
	const extension = filename.lastIndexOf(".");
	const filenameBase = extension > 0 ? filename.slice(0, extension) : filename;
	const directory = separator > 0 ? filePath.slice(0, separator) : "/";
	return Object.freeze({
		resolveVariable(name: string): string | undefined {
			switch (name) {
				case "TM_FILENAME": return filename;
				case "TM_FILENAME_BASE": return filenameBase;
				case "TM_DIRECTORY": return directory;
				case "TM_FILEPATH": return filePath;
				default: return undefined;
			}
		},
	});
}
