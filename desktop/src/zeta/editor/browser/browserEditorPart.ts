import { type Event } from "../../base/common/event.js";
import { DisposableStore } from "../../base/common/lifecycle.js";
import { BrowserTextMateService } from "../../workbench/services/textMate/browser/browserTextMateService.js";
import { type TextMateGrammarCatalog } from "../../workbench/services/textMate/common/textMateGrammarCatalog.js";
import { type TextMateGrammarDefinition } from "../../workbench/services/textMate/common/textMateGrammarRegistry.js";
import { type ITextMateService } from "../../workbench/services/textMate/common/textMateService.js";
import { type TextMateScopeThemeSource } from "../../workbench/services/textMate/common/textMateScopeTheme.js";
import { EditorPart, type EditorPartOptions } from "./editorPart.js";
import { createCompletionWorkerFactory } from "./language/languageCompletionWorkerClient.js";

/** Creates the product browser editor part with Workbench TextMate and completion workers. */
export interface BrowserEditorPartOptions extends EditorPartOptions {
  /** Shared Workbench TextMate service. Direct callers may omit it to get a private browser service. */
  readonly textMateService?: ITextMateService;
  /** Product or extension grammar contributions owned by this browser editor part. */
  readonly textMateGrammars?: readonly TextMateGrammarDefinition[];
  /** Caller-owned serializable scope theme; later revisions reanalyze this editor part. */
  readonly textMateScopeTheme?: TextMateScopeThemeSource;
}

/** Creates the product browser editor part with Workbench TextMate and completion workers. */
export function createBrowserEditorPart(options: BrowserEditorPartOptions): EditorPart {
  const textMateService = options.textMateService ?? new BrowserTextMateService(options.textMateGrammars, options.textMateScopeTheme);
  const ownsTextMateService = options.textMateService === undefined;
  const onDidChangeLanguageSupport: Event<void> = listener => {
    const subscriptions = new DisposableStore();
    subscriptions.add(textMateService.grammars.onDidChangeCatalog((_catalog: TextMateGrammarCatalog) => listener()));
    subscriptions.add(textMateService.scopeTheme.onDidChangeTheme(() => listener()));
    return subscriptions;
  };
  try {
    return new EditorPart({
      ...options,
      syntaxWorkerFactory: textMateService.syntaxWorkerFactory,
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
