import { Emitter, type Event } from "../../../../base/common/event.js";
import { DisposableOwner, DisposableStore } from "../../../../base/common/lifecycle.js";
import { parseLanguageConfiguration } from "../../../../editor/alpha/common/languages/languageConfigurationParser.js";
import type { ILanguageFeaturesService } from "../../../../editor/alpha/common/services/languageService.js";
import type { IExtensionApi, ExtensionCatalog, ExtensionDescriptor } from "../../../../platform/extensions/common/extensionApi.js";
import type { ITextMateService } from "../../textMate/common/textMateService.js";
import type { TextMateGrammarDefinition } from "../../textMate/common/textMateGrammarRegistry.js";
import { parseJsonc } from "../common/jsonc.js";
import { parseExtensionManifest } from "../common/extensionService.js";
import { createExtensionSnippetProvider, parseExtensionSnippetFile, type ExtensionSnippetDefinition } from "../common/extensionSnippetProvider.js";
import { ExtensionThemeRegistry, parseExtensionTheme, type ExtensionThemeDefinition } from "../common/extensionTheme.js";
import type { ExtensionServiceFailure, IExtensionService } from "../common/extensionService.js";

export interface AppServerExtensionServiceOptions {
  readonly api: IExtensionApi;
  readonly textMateService: ITextMateService;
  readonly languageFeaturesService?: ILanguageFeaturesService;
}

/** Loads Rust-discovered declarative extensions and projects their grammar contributions into TextMate. */
export class AppServerExtensionService extends DisposableOwner implements IExtensionService {
  private readonly changeEmitter = this.own(new Emitter<ExtensionCatalog>());
  private readonly failureEmitter = this.own(new Emitter<ExtensionServiceFailure>());
  private registrations = this.own(new DisposableStore());
  private catalog: ExtensionCatalog = Object.freeze({
    generation: 0,
    extensions: Object.freeze([]),
    diagnostics: Object.freeze([]),
  });
  private loading: Promise<void> | undefined;
  private disposed = false;
  readonly themes: ExtensionThemeRegistry;

  readonly onDidChange: Event<ExtensionCatalog> = this.changeEmitter.event;
  readonly onDidFail: Event<ExtensionServiceFailure> = this.failureEmitter.event;

  constructor(private readonly options: AppServerExtensionServiceOptions) {
    super();
    this.themes = this.own(new ExtensionThemeRegistry());
    if (!options || typeof options !== "object") {
      this.dispose();
      throw new TypeError("App Server extension service options are required");
    }
    if (!options.api || typeof options.api.list !== "function" || typeof options.api.readResource !== "function") {
      this.dispose();
      throw new TypeError("App Server extension service requires an extension API");
    }
    if (!options.textMateService || typeof options.textMateService.grammars.registerGrammar !== "function") {
      this.dispose();
      throw new TypeError("App Server extension service requires a TextMate service");
    }
  }

  get currentCatalog(): ExtensionCatalog {
    this.ensureAlive();
    return this.catalog;
  }

  start(): Promise<void> {
    this.ensureAlive();
    return this.reload();
  }

  reload(): Promise<void> {
    this.ensureAlive();
    if (this.loading) return this.loading;
    const operation = this.loadAndRegister();
    this.loading = operation;
    void operation.then(() => {
      if (this.loading === operation) this.loading = undefined;
    }, () => {
      if (this.loading === operation) this.loading = undefined;
    });
    return operation;
  }

  override dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    super.dispose();
  }

  private async loadAndRegister(): Promise<void> {
    const registrations = new DisposableStore();
    const languageConfigurations = new Map<string, Promise<ReturnType<typeof parseLanguageConfiguration>>>();
    const snippetFiles = new Map<string, Promise<readonly ExtensionSnippetDefinition[]>>();
    const themes: ExtensionThemeDefinition[] = [];
    let activeExtension: ExtensionDescriptor | undefined;
    try {
      const catalog = await this.options.api.list("refresh");
      for (const extension of catalog.extensions) {
        activeExtension = extension;
        const manifest = parseExtensionManifest(extension.manifestJson, extension);
        if ((manifest.contributes.languages.length > 0 || manifest.contributes.snippets.length > 0) && !this.options.languageFeaturesService) {
          throw new Error(`Extension '${extension.id}' contributes language features but no editor language service is available`);
        }
        for (const language of manifest.contributes.languages) {
          registrations.add(this.options.languageFeaturesService!.registerLanguage({
            id: language.id,
            aliases: language.aliases,
            extensions: language.extensions,
            filenames: language.filenames,
            filenamePatterns: language.filenamePatterns,
            mimetypes: language.mimetypes,
          }, { priority: 100 }));
          if (language.configuration !== undefined) {
            const key = `${extension.id}\0${language.configuration}`;
            const configuration = languageConfigurations.get(key) ?? this.loadLanguageConfiguration(extension.id, language.configuration);
            languageConfigurations.set(key, configuration);
            registrations.add(this.options.languageFeaturesService!.registerLanguageConfiguration(language.id, await configuration, { priority: 100 }));
          }
        }
        for (const [snippetIndex, snippet] of manifest.contributes.snippets.entries()) {
          const key = `${extension.id}\0${snippet.path}`;
          const definitions = snippetFiles.get(key) ?? this.loadSnippetFile(extension.id, snippet.path);
          snippetFiles.set(key, definitions);
          const parsed = await definitions;
          for (const languageId of snippet.language) {
            const providerSnippets = parsed.filter(candidate => candidate.prefixes.length > 0 && (!candidate.scopes || candidate.scopes.includes(languageId)));
            if (providerSnippets.length === 0) continue;
            registrations.add(this.options.languageFeaturesService!.registerCompletionProvider(createExtensionSnippetProvider(`${extension.id}.snippet.${snippetIndex}.${languageId}`, languageId, providerSnippets)));
          }
        }
        for (const [themeIndex, theme] of manifest.contributes.themes.entries()) {
          themes.push(await this.loadTheme(extension, themeIndex, theme.label, theme.path, theme.uiTheme));
        }
        for (const grammar of manifest.contributes.grammars) {
          const definition: TextMateGrammarDefinition = {
            scopeName: grammar.scopeName,
            ...(grammar.language === undefined ? {} : { languageId: grammar.language }),
            injectTo: grammar.injectTo,
            ...(grammar.embeddedLanguages === undefined ? {} : { embeddedLanguages: grammar.embeddedLanguages }),
            ...(grammar.tokenTypes === undefined ? {} : { tokenTypes: grammar.tokenTypes }),
            ...(grammar.balancedBracketScopes === undefined ? {} : { balancedBracketScopes: grammar.balancedBracketScopes }),
            ...(grammar.unbalancedBracketScopes === undefined ? {} : { unbalancedBracketScopes: grammar.unbalancedBracketScopes }),
            filePath: grammar.path,
            loadGrammar: () => this.loadGrammar(extension.id, grammar.path),
          };
          registrations.add(this.options.textMateService.grammars.registerGrammar(definition));
        }
      }
      const previous = this.registrations;
      this.registrations = this.own(registrations);
      previous.dispose();
      this.themes.replace(themes);
      this.catalog = catalog;
      this.changeEmitter.fire(catalog);
    } catch (error) {
      registrations.dispose();
      this.failureEmitter.fire(Object.freeze({ extension: activeExtension, error }));
      throw error;
    }
  }

  private async loadGrammar(extensionId: string, path: string): Promise<string> {
    const bytes = await this.options.api.readResource(extensionId, path);
    try {
      return new TextDecoder("utf-8", { fatal: true }).decode(bytes);
    } catch {
      throw new TypeError(`Extension '${extensionId}' grammar '${path}' is not valid UTF-8`);
    }
  }

  private async loadLanguageConfiguration(extensionId: string, path: string): Promise<ReturnType<typeof parseLanguageConfiguration>> {
    const bytes = await this.options.api.readResource(extensionId, path);
    const text = new TextDecoder("utf-8", { fatal: true }).decode(bytes);
    return parseLanguageConfiguration(parseJsonc(text, `Extension '${extensionId}' language configuration '${path}'`), `Extension '${extensionId}' language configuration '${path}'`);
  }

  private async loadSnippetFile(extensionId: string, path: string): Promise<readonly ExtensionSnippetDefinition[]> {
    const bytes = await this.options.api.readResource(extensionId, path);
    const text = new TextDecoder("utf-8", { fatal: true }).decode(bytes);
    return parseExtensionSnippetFile(parseJsonc(text, `Extension '${extensionId}' snippet file '${path}'`), `Extension '${extensionId}' snippet file '${path}'`);
  }

  private async loadTheme(extension: ExtensionDescriptor, index: number, label: string, path: string, uiTheme: string | undefined): Promise<ExtensionThemeDefinition> {
    const bytes = await this.options.api.readResource(extension.id, path);
    const text = new TextDecoder("utf-8", { fatal: true }).decode(bytes);
    return parseExtensionTheme(parseJsonc(text, `Extension '${extension.id}' theme '${path}'`), `${extension.id}:${index}`, extension.id, label, uiTheme, `Extension '${extension.id}' theme '${path}'`);
  }

  private ensureAlive(): void {
    if (this.disposed) throw new ReferenceError("AppServerExtensionService is already disposed");
  }
}
