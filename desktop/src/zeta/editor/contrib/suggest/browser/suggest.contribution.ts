import { registerEditorContribution } from "../../../browser/editorContribution.js";
import { type EditorResourceInput } from "../../../common/editorResource.js";
import { LanguageCompletionSessionController } from "../common/suggestModel.js";
import "./suggestWidget.js";

registerEditorContribution({
  id: "editor.contrib.suggest",
  configure: context => {
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
    context.setTextInputCompletion({
      session,
      requests: {
        service: completions,
        languageId: context.languageId,
        onRequestError: context.onLanguageError,
      },
    });
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
