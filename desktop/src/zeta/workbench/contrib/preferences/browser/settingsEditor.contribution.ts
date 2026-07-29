import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import type { IConfigurationService } from "../../../../platform/configuration/common/configuration.js";
import type { IDialogService } from "../../../../platform/dialogs/common/dialogs.js";
import type { IThemeService } from "../../../../platform/theme/common/themeService.js";
import { ModalEditorPart } from "../../../browser/parts/editor/modalEditorPart.js";
import type { IUserThemeService } from "../../../common/userThemes.js";
import type { ISettingsService } from "../../../services/preferences/common/settings.js";
import { SettingsEditor } from "./settingsEditor.js";

export interface SettingsEditorContributionOptions {
  readonly configurationService: IConfigurationService;
  readonly container: HTMLElement;
  readonly dialogService: IDialogService;
  readonly settingsService: ISettingsService;
  readonly themeService: IThemeService;
  readonly userThemeService: IUserThemeService;
}

/** Connects window Settings state to its modal editor host and content. */
export class SettingsEditorContribution extends DisposableOwner {
  readonly #editor: SettingsEditor;
  readonly #modalEditor: ModalEditorPart;

  constructor(options: SettingsEditorContributionOptions) {
    super();
    this.#editor = this.own(new SettingsEditor({
      ownerDocument: options.container.ownerDocument,
      configurationService: options.configurationService,
      dialogService: options.dialogService,
      settingsService: options.settingsService,
      themeService: options.themeService,
      userThemeService: options.userThemeService,
    }));
    this.#modalEditor = this.own(new ModalEditorPart({
      container: options.container,
      title: "Zeta Settings",
      content: this.#editor.element,
      focusContent: () => this.#editor.focus(),
    }));
    this.#modalEditor.element.classList.add("zeta-settings-modal");

    this.own(this.#modalEditor.onDidRequestClose(() => {
      options.settingsService.close();
    }));
    this.own(options.settingsService.onDidChangeVisibility((visible) => {
      if (visible) this.#show();
      else {
        this.#editor.cancelThemeEditing();
        this.#modalEditor.hide();
      }
    }));
    if (options.settingsService.isOpen) this.#show();
  }

  #show(): void {
    this.#modalEditor.show();
    this.#editor.layout();
  }
}
