import { type Event } from "../../../../base/common/event.js";
import { DisposableStore } from "../../../../base/common/lifecycle.js";
import { EditorBrowser, type EditorBrowserOptions } from "../../../../editor/browser/editorBrowser.js";
import { createCompletionWorkerFactory } from "../../../../editor/browser/language/languageCompletionWorkerClient.js";
import { BrowserTextMateService } from "../../../services/textMate/browser/browserTextMateService.js";
import { type TextMateGrammarCatalog } from "../../../services/textMate/common/textMateGrammarCatalog.js";
import { type TextMateGrammarDefinition } from "../../../services/textMate/common/textMateGrammarRegistry.js";
import { type ITextMateService } from "../../../services/textMate/common/textMateService.js";
import { type TextMateScopeThemeSource } from "../../../services/textMate/common/textMateScopeTheme.js";

/** Creates the product browser editor part with Workbench TextMate and completion workers. */
export interface BrowserEditorPartOptions extends EditorBrowserOptions {
	/** Shared Workbench TextMate service. Direct callers may omit it to get a private browser service. */
	readonly textMateService?: ITextMateService;
	/** Product or extension grammar contributions owned by this browser editor part. */
	readonly textMateGrammars?: readonly TextMateGrammarDefinition[];
	/** Caller-owned serializable scope theme; later revisions reanalyze this editor part. */
	readonly textMateScopeTheme?: TextMateScopeThemeSource;
}

/** Creates the product browser editor part with Workbench TextMate and completion workers. */
export function createBrowserEditorPart(options: BrowserEditorPartOptions): EditorBrowser {
	const textMateService = options.textMateService ?? new BrowserTextMateService(options.textMateGrammars, options.textMateScopeTheme);
	const ownsTextMateService = options.textMateService === undefined;
	const onDidChangeLanguageSupport: Event<void> = listener => {
		const subscriptions = new DisposableStore();
		subscriptions.add(textMateService.grammars.onDidChangeCatalog((_catalog: TextMateGrammarCatalog) => listener()));
		subscriptions.add(textMateService.scopeTheme.onDidChangeTheme(() => listener()));
		return subscriptions;
	};
	try {
		return new EditorBrowser({
			...options,
			syntaxWorkerFactory: textMateService.syntaxWorkerFactory,
			...(options.languageFeaturesService ? {} : { completionWorkerFactory: createCompletionWorkerFactory() }),
			...(ownsTextMateService ? { languageSupport: textMateService } : {}),
			onDidChangeLanguageSupport,
		});
	} catch (error) {
		if (ownsTextMateService) textMateService.dispose();
		throw error;
	}
}
