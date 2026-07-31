import { type Event } from "../../../base/common/event.js";
import { type ISyntaxAnalysisService } from "../../../platform/syntax/common/syntaxAnalysisService.js";
import { BrowserTextMateAnalysisWorkerSupport } from "../../textmate/browser/textMateAnalysisWorkerClient.js";
import { type TextMateGrammarCatalog } from "../../textmate/common/textMateGrammarCatalog.js";
import { AlphaEditorSession, type AlphaEditorSessionOptions } from "./alphaEditorSession.js";
import { createSyntaxAnalysisServiceAdapter } from "./syntaxAnalysisServiceAdapter.js";
import { createAlphaCompletionWorkerFactory } from "./languageCompletionWorkerClient.js";

/** Creates Alpha's product browser session with service-backed Rust syntax, TextMate, and completion workers. */
export function createBrowserAlphaEditorSession(options: AlphaEditorSessionOptions, syntaxAnalysisService: ISyntaxAnalysisService): AlphaEditorSession {
  const languageSupport = new BrowserTextMateAnalysisWorkerSupport();
  const onDidChangeLanguageSupport: Event<void> = listener => languageSupport.grammars.onDidChangeCatalog((_catalog: TextMateGrammarCatalog) => listener());
  try {
    return new AlphaEditorSession({
      ...options,
      analysisWorkerFactory: createSyntaxAnalysisServiceAdapter(syntaxAnalysisService, options.modelReference.resource.toString(), options.languageId, languageSupport.workerFactory),
      completionWorkerFactory: createAlphaCompletionWorkerFactory(),
      languageSupport,
      onDidChangeLanguageSupport,
      whenLanguageSupportReady: () => languageSupport.grammars.whenReady(),
    });
  } catch (error) {
    languageSupport.dispose();
    throw error;
  }
}
