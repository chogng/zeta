import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { ModalEditorPart } from "../../../browser/parts/editor/modalEditorPart.js";
import type { ISettingsService } from "../../../services/preferences/common/settings.js";
import { SettingsEditor } from "./settingsEditor.js";

export interface SettingsEditorContributionOptions {
  readonly container: HTMLElement;
  readonly settingsService: ISettingsService;
}

/** Connects window Settings state to its modal editor host and content. */
export class SettingsEditorContribution extends DisposableOwner {
  readonly #editor: SettingsEditor;
  readonly #modalEditor: ModalEditorPart;

  constructor(options: SettingsEditorContributionOptions) {
    super();
    this.#editor = this.own(new SettingsEditor({
      ownerDocument: options.container.ownerDocument,
      settingsService: options.settingsService,
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
      else this.#modalEditor.hide();
    }));
    if (options.settingsService.isOpen) this.#show();
  }

  #show(): void {
    this.#modalEditor.show();
    this.#editor.layout();
  }
}
