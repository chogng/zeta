import type { EditorResourceInput } from "../../../browser/editorBrowser.js";
import { registerTextEditorCapabilityContribution, type EditorCapability } from "../../../browser/editorExtensions.js";
import { LanguageCompletionService } from "../../../common/languages/completion/languageCompletionService.js";
import { isCompletionsEnablementEnabled } from "../../../common/services/ownedCompletionsEnablement.js";
import { LanguageCompletionSessionController } from "../common/languageCompletionSessionController.js";
import { SuggestController } from "./suggestController.js";

interface SuggestContributionState {
	readonly service: LanguageCompletionService;
	readonly session: LanguageCompletionSessionController;
}

const suggestState: EditorCapability<SuggestContributionState> = {
	id: "editor.suggest.state",
};

registerTextEditorCapabilityContribution({
	id: "editor.contrib.suggest",
	configure: context => {
		if (context.options.suggestions !== undefined && !isCompletionsEnablementEnabled(context.options.suggestions, context.languageId)) return;
		const completions = context.register(new LanguageCompletionService(context.model, context.languageFeaturesService.completionProvider, {
			resource: context.options.input.resource,
			...(context.options.completionWorkerFactory ? { workerFactory: context.options.completionWorkerFactory } : {}),
		}));
		const session = context.register(new LanguageCompletionSessionController(completions.results, context.selections, {
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
		context.register(new SuggestController(
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
