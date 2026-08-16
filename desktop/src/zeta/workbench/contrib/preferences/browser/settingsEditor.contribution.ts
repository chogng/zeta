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
import { SettingsEditor } from "./settingsEditor.js";

export interface SettingsEditorContributionOptions {
  readonly configurationService: IConfigurationService;
  readonly container: HTMLElement;
  readonly dialogService: IDialogService;
  readonly settingsService: ISettingsService;
  readonly themeService: IThemeService;
  readonly userThemeService: IUserThemeService;
  readonly codeIndexService?: ICodeIndexService;
  readonly connectorService?: IConnectorService;
  readonly pluginService?: IPluginService;
  readonly marketplaceService?: IMarketplaceService;
  readonly toolSearchService?: IToolSearchService;
}

/** Connects window Settings state to its modal editor host and content. */
export class SettingsEditorContribution extends DisposableOwner {
  private readonly editor: SettingsEditor;
  private readonly modalEditor: ModalEditorPart;

  constructor(options: SettingsEditorContributionOptions) {
    super();
    this.editor = this.own(new SettingsEditor({
      ownerDocument: options.container.ownerDocument,
      configurationService: options.configurationService,
      dialogService: options.dialogService,
      settingsService: options.settingsService,
      themeService: options.themeService,
      userThemeService: options.userThemeService,
      codeIndexService: options.codeIndexService ?? unavailableCodeIndexService,
      connectorService: options.connectorService ?? unavailableConnectorService,
      pluginService: options.pluginService ?? unavailablePluginService,
      marketplaceService: options.marketplaceService ?? unavailableMarketplaceService,
      toolSearchService: options.toolSearchService ?? unavailableToolSearchService,
    }));
    this.modalEditor = this.own(new ModalEditorPart({
      container: options.container,
      title: "Zeta Settings",
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
