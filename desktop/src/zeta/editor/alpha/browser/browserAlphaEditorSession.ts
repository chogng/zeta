import { type Event } from "../../../base/common/event.js";
import { DisposableStore } from "../../../base/common/lifecycle.js";
import { BrowserTextMateAnalysisWorkerSupport } from "../../textmate/browser/textMateAnalysisWorkerClient.js";
import { type TextMateGrammarCatalog } from "../../textmate/common/textMateGrammarCatalog.js";
import { type TextMateGrammarDefinition } from "../../textmate/common/textMateGrammarRegistry.js";
import { type TextMateScopeThemeSource } from "../../textmate/common/textMateScopeTheme.js";
import { AlphaEditorSession, type AlphaEditorSessionOptions } from "./alphaEditorSession.js";
import { createAlphaCompletionWorkerFactory } from "./languageCompletionWorkerClient.js";

/** Creates Alpha's product browser session with editor-owned TextMate and completion workers. */
export interface BrowserAlphaEditorSessionOptions extends AlphaEditorSessionOptions {
  /** Product or extension grammar contributions owned by this browser session. */
  readonly textMateGrammars?: readonly TextMateGrammarDefinition[];
  /** Caller-owned serializable scope theme; later revisions reanalyze this session. */
  readonly textMateScopeTheme?: TextMateScopeThemeSource;
}

/** Creates Alpha's product browser session with editor-owned TextMate and completion workers. */
export function createBrowserAlphaEditorSession(options: BrowserAlphaEditorSessionOptions): AlphaEditorSession {
  const languageSupport = new BrowserTextMateAnalysisWorkerSupport(options.textMateGrammars, options.textMateScopeTheme);
  const onDidChangeLanguageSupport: Event<void> = listener => {
    const subscriptions = new DisposableStore();
    subscriptions.add(languageSupport.grammars.onDidChangeCatalog((_catalog: TextMateGrammarCatalog) => listener()));
    subscriptions.add(languageSupport.scopeTheme.onDidChangeTheme(() => listener()));
    return subscriptions;
  };
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
