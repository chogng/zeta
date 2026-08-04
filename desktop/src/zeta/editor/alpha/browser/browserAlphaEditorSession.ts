import { type Event } from "../../../base/common/event.js";
import { DisposableStore } from "../../../base/common/lifecycle.js";
import { BrowserTextMateService } from "../../../workbench/services/textMate/browser/browserTextMateService.js";
import { type TextMateGrammarCatalog } from "../../../workbench/services/textMate/common/textMateGrammarCatalog.js";
import { type TextMateGrammarDefinition } from "../../../workbench/services/textMate/common/textMateGrammarRegistry.js";
import { type ITextMateService } from "../../../workbench/services/textMate/common/textMateService.js";
import { type TextMateScopeThemeSource } from "../../../workbench/services/textMate/common/textMateScopeTheme.js";
import { AlphaEditorSession, type AlphaEditorSessionOptions } from "./alphaEditorSession.js";
import { createCompletionWorkerFactory } from "../browser/language/languageCompletionWorkerClient.js";

/** Creates Alpha's product browser session with Workbench TextMate and Alpha completion workers. */
export interface BrowserAlphaEditorSessionOptions extends AlphaEditorSessionOptions {
  /** Shared Workbench TextMate service. Direct callers may omit it to get a private browser service. */
  readonly textMateService?: ITextMateService;
  /** Product or extension grammar contributions owned by this browser session. */
  readonly textMateGrammars?: readonly TextMateGrammarDefinition[];
  /** Caller-owned serializable scope theme; later revisions reanalyze this session. */
  readonly textMateScopeTheme?: TextMateScopeThemeSource;
}

/** Creates Alpha's product browser session with Workbench TextMate and Alpha completion workers. */
export function createBrowserAlphaEditorSession(options: BrowserAlphaEditorSessionOptions): AlphaEditorSession {
  const textMateService = options.textMateService ?? new BrowserTextMateService(options.textMateGrammars, options.textMateScopeTheme);
  const ownsTextMateService = options.textMateService === undefined;
  const onDidChangeLanguageSupport: Event<void> = listener => {
    const subscriptions = new DisposableStore();
    subscriptions.add(textMateService.grammars.onDidChangeCatalog((_catalog: TextMateGrammarCatalog) => listener()));
    subscriptions.add(textMateService.scopeTheme.onDidChangeTheme(() => listener()));
    return subscriptions;
  };
  try {
    return new AlphaEditorSession({
      ...options,
      analysisWorkerFactory: textMateService.analysisWorkerFactory,
      completionWorkerFactory: createCompletionWorkerFactory(),
      ...(ownsTextMateService ? { languageSupport: textMateService } : {}),
      onDidChangeLanguageSupport,
      whenLanguageSupportReady: () => textMateService.grammars.whenReady(),
    });
  } catch (error) {
    if (ownsTextMateService) textMateService.dispose();
    throw error;
  }
}
