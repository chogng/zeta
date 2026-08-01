import { type Event } from "../../../base/common/event.js";
import { BrowserTextMateAnalysisWorkerSupport } from "../../textmate/browser/textMateAnalysisWorkerClient.js";
import { type TextMateGrammarCatalog } from "../../textmate/common/textMateGrammarCatalog.js";
import { AlphaEditorSession, type AlphaEditorSessionOptions } from "./alphaEditorSession.js";
import { createAlphaCompletionWorkerFactory } from "./languageCompletionWorkerClient.js";

/** Creates Alpha's product browser session with editor-owned TextMate and completion workers. */
export function createBrowserAlphaEditorSession(options: AlphaEditorSessionOptions): AlphaEditorSession {
  const languageSupport = new BrowserTextMateAnalysisWorkerSupport();
  const onDidChangeLanguageSupport: Event<void> = listener => languageSupport.grammars.onDidChangeCatalog((_catalog: TextMateGrammarCatalog) => listener());
  try {
    return new AlphaEditorSession({
      ...options,
      analysisWorkerFactory: languageSupport.workerFactory,
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
