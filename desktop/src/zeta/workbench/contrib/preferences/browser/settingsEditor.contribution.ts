import type { IContextViewProvider } from "../../../../base/browser/ui/contextview/contextview.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import type { IConfigurationService } from "../../../../platform/configuration/common/configurationService.js";
import type { IDialogService } from "../../../../platform/dialogs/common/dialogs.js";
import type { IThemeService } from "../../../../platform/theme/common/themeService.js";
import { ModalEditorPart } from "../../../browser/parts/editor/modalEditorPart.js";
import type { IUserThemeService } from "../../../common/userThemes.js";
import type { ISettingsService } from "../../../services/preferences/common/settings.js";
import type { ICodeIndexService } from "../../../../platform/codeIndex/common/codeIndexService.js";
import type { IToolSearchService } from "../../../../platform/toolSearch/common/toolSearchService.js";
import type { IConnectorService } from "../../../../platform/connectors/common/connectorService.js";
import type { IPluginService } from "../../../../platform/plugins/common/pluginService.js";
import type { IMarketplaceService } from "../../../../platform/marketplace/common/marketplaceService.js";
import type { ILanguagePackService } from "../../../../platform/languagePacks/common/languagePacksService.js";
import type { ILocaleService } from "../../../services/localization/common/locale.js";
import type { ILocalizationService } from "../../../services/localization/common/localizationService.js";
import type { IWorkspaceContextService } from "../../../../platform/workspace/common/workspace.js";
import type { IWorkspaceTrustService } from "../../../../platform/workspaceTrust/common/workspaceTrustService.js";
import type { IWorkspaceOpenService } from "../../../services/workspaces/browser/workspaceOpenService.js";
import { SettingsEditor } from "./settingsEditor.js";

export interface SettingsEditorContributionOptions {
  readonly configurationService: IConfigurationService;
  readonly container: HTMLElement;
  readonly contextViewProvider: IContextViewProvider;
  readonly dialogService: IDialogService;
  readonly settingsService: ISettingsService;
  readonly themeService: IThemeService;
  readonly userThemeService: IUserThemeService;
  readonly codeIndexService?: ICodeIndexService;
  readonly connectorService?: IConnectorService;
  readonly pluginService?: IPluginService;
  readonly marketplaceService?: IMarketplaceService;
  readonly languagePackService?: ILanguagePackService;
  readonly localeService?: ILocaleService;
  readonly localizationService?: ILocalizationService;
  readonly toolSearchService?: IToolSearchService;
  readonly workspaceTrustService?: IWorkspaceTrustService;
  readonly workspaceOpenService?: IWorkspaceOpenService;
  readonly workspaceContextService?: IWorkspaceContextService;
}

/** Connects window Settings state to its modal editor host and content. */
export class SettingsEditorContribution extends DisposableOwner {
  private readonly editor: SettingsEditor;
  private readonly modalEditor: ModalEditorPart;

  constructor(options: SettingsEditorContributionOptions) {
    super();
    this.editor = this.own(new SettingsEditor(options.container, {
      contextViewProvider: options.contextViewProvider,
      configurationService: options.configurationService,
      dialogService: options.dialogService,
      settingsService: options.settingsService,
      themeService: options.themeService,
      userThemeService: options.userThemeService,
      codeIndexService: options.codeIndexService ?? unavailableCodeIndexService,
      connectorService: options.connectorService ?? unavailableConnectorService,
      pluginService: options.pluginService ?? unavailablePluginService,
      marketplaceService: options.marketplaceService ?? unavailableMarketplaceService,
      languagePackService: options.languagePackService ?? unavailableLanguagePackService,
      localeService: options.localeService ?? unavailableLocaleService,
      localizationService: options.localizationService ?? unavailableLocalizationService,
      toolSearchService: options.toolSearchService ?? unavailableToolSearchService,
      workspaceTrustService: options.workspaceTrustService ?? unavailableWorkspaceTrustService,
      workspaceOpenService: options.workspaceOpenService ?? unavailableWorkspaceOpenService,
      workspaceContextService: options.workspaceContextService,
    }));
    this.modalEditor = this.own(new ModalEditorPart({
      container: options.container,
      title: (options.localizationService ?? unavailableLocalizationService).translate("zeta.settings", "chrome.modalTitle", "Zeta Settings"),
      content: this.editor.element,
      focusContent: () => this.editor.focus(),
    }));
    this.modalEditor.element.classList.add("zeta-settings-modal");

    this.own(this.modalEditor.onDidRequestClose(() => {
      options.settingsService.close();
    }));
    this.own(options.settingsService.onDidChangeVisibility((visible) => {
      if (visible) this.show();
      else {
        this.editor.cancelThemeEditing();
        this.modalEditor.hide();
      }
    }));
    if (options.settingsService.isOpen) this.show();
  }

  private show(): void {
    this.modalEditor.show();
    this.editor.layout();
  }
}

const unavailableCodeIndexService: ICodeIndexService = {
  readConfig: () => Promise.reject(new Error("Code index settings are unavailable.")),
  configureProvider: () => Promise.reject(new Error("Code index settings are unavailable.")),
  configure: () => Promise.reject(new Error("Code index settings are unavailable.")),
  authorize: () => Promise.reject(new Error("Code index settings are unavailable.")),
  revoke: () => Promise.reject(new Error("Code index settings are unavailable.")),
  status: () => Promise.reject(new Error("Code index settings are unavailable.")),
  cancel: () => Promise.reject(new Error("Code index settings are unavailable.")),
  retry: () => Promise.reject(new Error("Code index settings are unavailable.")),
};

const unavailableToolSearchService: IToolSearchService = {
  readConfig: () => Promise.reject(new Error("Tool Search settings are unavailable.")),
  configure: () => Promise.reject(new Error("Tool Search settings are unavailable.")),
};

const unavailableWorkspaceTrustService: IWorkspaceTrustService = {
  list: () => Promise.reject(new Error("Workspace Trust settings are unavailable.")),
  read: () => Promise.reject(new Error("Workspace Trust settings are unavailable.")),
  set: () => Promise.reject(new Error("Workspace Trust settings are unavailable.")),
  forget: () => Promise.reject(new Error("Workspace Trust settings are unavailable.")),
};

const unavailableWorkspaceOpenService: IWorkspaceOpenService = {
  canOpenFolder: false,
  canOpenWorkspace: false,
  openFolder: () => Promise.reject(new Error("Folder picking is unavailable.")),
  openWorkspace: () => Promise.reject(new Error("Workspace opening is unavailable.")),
  pickFolder: () => Promise.reject(new Error("Folder picking is unavailable.")),
};

const unavailableConnectorService: IConnectorService = {
  onDidChange: () => ({ dispose() {}, [Symbol.dispose]() {} }),
  list: () => Promise.reject(new Error("Connectors are unavailable.")),
  connectApiToken: () => Promise.reject(new Error("Connectors are unavailable.")),
  connectOAuth: () => Promise.reject(new Error("Connectors are unavailable.")),
  disconnect: () => Promise.reject(new Error("Connectors are unavailable.")),
  refreshOAuth: () => Promise.reject(new Error("Connectors are unavailable.")),
  revokeOAuth: () => Promise.reject(new Error("Connectors are unavailable.")),
};

const unavailablePluginService: IPluginService = {
  onDidChange: () => ({ dispose() {}, [Symbol.dispose]() {} }),
  list: () => Promise.reject(new Error("Plugins are unavailable.")),
  enable: () => Promise.reject(new Error("Plugins are unavailable.")),
  disable: () => Promise.reject(new Error("Plugins are unavailable.")),
  grant: () => Promise.reject(new Error("Plugins are unavailable.")),
  revokeGrant: () => Promise.reject(new Error("Plugins are unavailable.")),
  uninstall: () => Promise.reject(new Error("Plugins are unavailable.")),
};

const unavailableMarketplaceService: IMarketplaceService = {
  onDidChangeInstalled: () => ({ dispose() {}, [Symbol.dispose]() {} }),
  cachedBrowse: () => undefined,
  browse: () => Promise.reject(new Error("Marketplace is unavailable.")),
  refreshBrowse: () => Promise.reject(new Error("Marketplace is unavailable.")),
  search: () => Promise.reject(new Error("Marketplace is unavailable.")),
  get: () => Promise.reject(new Error("Marketplace is unavailable.")),
  download: () => Promise.reject(new Error("Marketplace is unavailable.")),
  install: () => Promise.reject(new Error("Marketplace is unavailable.")),
  update: () => Promise.reject(new Error("Marketplace is unavailable.")),
  uninstall: () => Promise.reject(new Error("Marketplace is unavailable.")),
  listInstalled: () => Promise.reject(new Error("Marketplace is unavailable.")),
  acquireCapability: () => Promise.reject(new Error("Marketplace is unavailable.")),
  releaseCapability: () => Promise.reject(new Error("Marketplace is unavailable.")),
  openResource: () => Promise.reject(new Error("Marketplace is unavailable.")),
};

const unavailableLocalizationService: ILocalizationService = {
  onDidChange: () => ({ dispose() {}, [Symbol.dispose]() {} }),
  whenReady: Promise.resolve(),
  translate: (_bundle, _key, fallback) => fallback,
};

const unavailableLocaleService: ILocaleService = {
  locale: "en",
  onDidChangeLocale: () => ({ dispose() {}, [Symbol.dispose]() {} }),
  whenReady: Promise.resolve(),
  setLocale: () => Promise.reject(new Error("Locale selection is unavailable.")),
};

const unavailableLanguagePackService: ILanguagePackService = {
  onDidChange: () => ({ dispose() {}, [Symbol.dispose]() {} }),
  whenReady: Promise.resolve(),
  catalogs: [],
  availableLocales: [],
  installedPackages: [],
  search: () => Promise.reject(new Error("Language packs are unavailable.")),
  install: () => Promise.reject(new Error("Language packs are unavailable.")),
  refresh: () => Promise.reject(new Error("Language packs are unavailable.")),
};
