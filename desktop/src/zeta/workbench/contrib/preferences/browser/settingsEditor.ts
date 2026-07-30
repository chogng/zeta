import "./media/settingsEditor.css";
import { addDisposableListener, stopEvent } from "../../../../base/browser/dom.js";
import { InputBox } from "../../../../base/browser/ui/inputbox/inputbox.js";
import { ScrollableElement } from "../../../../base/browser/ui/scrollbar/scrollableElement.js";
import { DisposableOwner, ResettableDisposableGroup } from "../../../../base/common/lifecycle.js";
import type { IConfigurationService } from "../../../../platform/configuration/common/configuration.js";
import type { IDialogService } from "../../../../platform/dialogs/common/dialogs.js";
import { ColorId, darkColorTheme, type IColorTheme, lightColorTheme } from "../../../../platform/theme/common/colorTheme.js";
import type { IThemeService } from "../../../../platform/theme/common/themeService.js";
import { isDarkColorScheme } from "../../../../platform/theme/common/theme.js";
import { parseUserColorTheme, serializeUserColorThemeDraft } from "../../../../platform/theme/common/userColorTheme.js";
import { WorkbenchConfiguration } from "../../../common/configuration.js";
import { SystemColorThemePreference, WorkbenchThemesRegistry } from "../../../common/theme.js";
import type { IUserThemeService } from "../../../common/userThemes.js";
import type { ISettingsService } from "../../../services/preferences/common/settings.js";
import { getSettingsSection, SettingsSections, type SettingsSectionDescriptor } from "../common/settingsSections.js";

export interface SettingsEditorOptions {
  readonly ownerDocument: Document;
  readonly configurationService: IConfigurationService;
  readonly dialogService: IDialogService;
  readonly settingsService: ISettingsService;
  readonly themeService: IThemeService;
  readonly userThemeService: IUserThemeService;
}

let nextSettingsEditorId = 1;

/** Search, navigation, and page content hosted by the Workbench modal editor. */
export class SettingsEditor extends DisposableOwner {
  readonly element: HTMLDivElement;
  private readonly configurationService: IConfigurationService;
  private readonly dialogService: IDialogService;
  private readonly settingsService: ISettingsService;
  private readonly themeService: IThemeService;
  private readonly userThemeService: IUserThemeService;
  private readonly searchInput: InputBox;
  private readonly navigationItems = new Map<string, HTMLButtonElement>();
  private readonly navigationEmpty: HTMLParagraphElement;
  private readonly navigationScrollable: ScrollableElement;
  private readonly contentScrollable: ScrollableElement;
  private readonly content: HTMLElement;
  private readonly contentHeading: HTMLHeadingElement;
  private readonly contentDescription: HTMLParagraphElement;
  private readonly sectionContent: HTMLDivElement;
  private readonly sectionBindings = this.own(new ResettableDisposableGroup());
  private themeDraft: ThemeDraft | undefined;
  private themeMessage = "";

  constructor(options: SettingsEditorOptions) {
    super();
    this.configurationService = options.configurationService;
    this.dialogService = options.dialogService;
    this.settingsService = options.settingsService;
    this.themeService = options.themeService;
    this.userThemeService = options.userThemeService;
    const editorId = `zeta-settings-editor-${nextSettingsEditorId++}`;
    this.element = options.ownerDocument.createElement("div");
    this.element.className = "zeta-settings-editor";

    const search = options.ownerDocument.createElement("div");
    search.className = "zeta-settings-search";
    search.setAttribute("role", "search");
    this.searchInput = this.own(new InputBox({
      ownerDocument: options.ownerDocument,
      type: "search",
      placeholder: "Search settings",
      ariaLabel: "Search settings",
      ariaControls: `${editorId}-navigation`,
    }));
    this.searchInput.element.classList.add("zeta-settings-search-input");
    search.append(this.searchInput.element);

    const layout = options.ownerDocument.createElement("div");
    layout.className = "zeta-settings-layout";

    const navigation = options.ownerDocument.createElement("nav");
    navigation.className = "zeta-settings-sidebar";
    navigation.setAttribute("aria-label", "Settings categories");
    this.navigationScrollable = this.own(new ScrollableElement({
      ownerDocument: options.ownerDocument,
      direction: "vertical",
      vertical: "auto",
      tabIndex: -1,
      wheel: { consume: "when-scrolling" },
    }));
    this.navigationScrollable.element.classList.add("zeta-settings-sidebar-scrollable");
    const navigationList = options.ownerDocument.createElement("ul");
    navigationList.className = "zeta-settings-navigation-list";
    navigationList.id = `${editorId}-navigation`;
    for (const section of SettingsSections) {
      const item = options.ownerDocument.createElement("li");
      const button = options.ownerDocument.createElement("button");
      button.className = "zeta-settings-navigation-item";
      button.type = "button";
      button.dataset.settingsSectionId = section.id;
      button.textContent = section.label;
      this.navigationItems.set(section.id, button);
      this.own(addDisposableListener(button, "click", () => {
        this.settingsService.open(section.id);
      }));
      this.own(addDisposableListener(button, "keydown", (event: KeyboardEvent) => {
        this.handleNavigationKeydown(event, section.id);
      }));
      item.append(button);
      navigationList.append(item);
    }
    this.navigationEmpty = options.ownerDocument.createElement("p");
    this.navigationEmpty.className = "zeta-settings-navigation-empty";
    this.navigationEmpty.textContent = "No settings found.";
    this.navigationEmpty.setAttribute("role", "status");
    this.navigationEmpty.hidden = true;
    this.navigationScrollable.append(navigationList, this.navigationEmpty);
    navigation.append(this.navigationScrollable.element);

    this.content = options.ownerDocument.createElement("main");
    this.content.className = "zeta-settings-page";
    this.content.dataset.settingsContainer = "";
    this.content.tabIndex = -1;
    this.contentScrollable = this.own(new ScrollableElement({
      ownerDocument: options.ownerDocument,
      direction: "vertical",
      vertical: "auto",
      tabIndex: -1,
      wheel: { consume: "when-scrolling" },
    }));
    this.contentScrollable.element.classList.add("zeta-settings-page-scrollable");
    const contentInner = options.ownerDocument.createElement("div");
    contentInner.className = "zeta-settings-page-inner";
    this.contentHeading = options.ownerDocument.createElement("h3");
    this.contentHeading.id = `${editorId}-section`;
    this.content.setAttribute("aria-labelledby", this.contentHeading.id);
    this.contentDescription = options.ownerDocument.createElement("p");
    this.contentDescription.className = "zeta-settings-description";
    this.sectionContent = options.ownerDocument.createElement("div");
    this.sectionContent.className = "zeta-settings-section-content";
    this.sectionContent.dataset.settingsSectionContent = "";
    contentInner.append(this.contentHeading, this.contentDescription, this.sectionContent);
    this.contentScrollable.append(contentInner);
    this.content.append(this.contentScrollable.element);

    layout.append(navigation, this.content);
    this.element.append(search, layout);
    this.renderSection(getSettingsSection(this.settingsService.activeSectionId));

    this.own(this.settingsService.onDidChangeActiveSection((sectionId) => {
      this.renderSection(getSettingsSection(sectionId));
    }));
    this.own(this.configurationService.onDidChangeConfiguration((event) => {
      if (
        event.affectsConfiguration(WorkbenchConfiguration.colorTheme) &&
        this.settingsService.activeSectionId === "appearance"
      ) {
        this.renderAppearance();
      }
    }));
    this.own(this.searchInput.onDidChange((value) => {
      this.filterNavigation(value);
    }));
    this.own(this.searchInput.onKeyDown((event) => {
      if (event.key === "Escape" && this.searchInput.value) {
        stopEvent(event);
        this.searchInput.value = "";
        return;
      }
      if (event.key !== "ArrowDown") return;
      const firstVisible = this.visibleNavigationSections()[0];
      if (!firstVisible) return;
      stopEvent(event);
      this.navigationItems.get(firstVisible.id)?.focus();
    }));
    this.defer(() => {
      if (this.themeDraft) this.themeService.setColorTheme(this.themeDraft.originalTheme);
      this.element.remove();
    });
  }

  focus(): void {
    this.searchInput.focus();
  }

  layout(): void {
    this.navigationScrollable.layout();
    this.contentScrollable.layout();
  }

  cancelThemeEditing(): void {
    if (!this.themeDraft) return;
    this.themeService.setColorTheme(this.themeDraft.originalTheme);
    this.themeDraft = undefined;
    this.themeMessage = "";
    if (this.settingsService.activeSectionId === "appearance") this.renderAppearance();
  }

  private renderSection(section: SettingsSectionDescriptor): void {
    for (const [sectionId, item] of this.navigationItems) {
      const active = sectionId === section.id;
      item.classList.toggle("is-active", active);
      if (active) item.setAttribute("aria-current", "page");
      else item.removeAttribute("aria-current");
    }
    this.content.dataset.activeSettingsSection = section.id;
    this.contentHeading.textContent = section.label;
    this.contentDescription.textContent = section.description;
    this.sectionBindings.clear();
    this.sectionContent.replaceChildren();
    if (section.id === "appearance") this.renderAppearance();
    this.contentScrollable.scrollTo(0, 0);
    this.contentScrollable.layout();
  }

  private renderAppearance(): void {
    this.sectionBindings.clear();
    const document = this.element.ownerDocument;
    const group = document.createElement("fieldset");
    group.className = "zeta-theme-setting";
    const legend = document.createElement("legend");
    legend.textContent = "Color theme";
    const hint = document.createElement("p");
    hint.className = "zeta-theme-setting-hint";
    hint.textContent = "Choose an appearance or keep Zeta synchronized with your operating system.";
    const options = document.createElement("div");
    options.className = "zeta-theme-options";
    const preference = this.configurationService.getValue(WorkbenchConfiguration.colorTheme);
    for (const descriptor of themeOptions(this.userThemeService)) {
      const label = document.createElement("label");
      label.className = "zeta-theme-option";
      label.dataset.themePreference = descriptor.value;
      const input = document.createElement("input");
      input.type = "radio";
      input.name = "zeta-color-theme";
      input.value = descriptor.value;
      input.checked = preference === descriptor.value;
      const preview = document.createElement("span");
      preview.className = "zeta-theme-preview";
      applyThemePreview(preview, descriptor.previewThemes);
      preview.setAttribute("aria-hidden", "true");
      const copy = document.createElement("span");
      copy.className = "zeta-theme-option-copy";
      const title = document.createElement("span");
      title.className = "zeta-theme-option-title";
      title.textContent = descriptor.label;
      const description = document.createElement("span");
      description.className = "zeta-theme-option-description";
      description.textContent = descriptor.description;
      copy.append(title, description);
      label.append(input, preview, copy);
      this.sectionBindings.add(addDisposableListener(input, "change", () => {
        if (!input.checked) return;
        if (this.themeDraft) {
          this.themeService.setColorTheme(this.themeDraft.originalTheme);
          this.themeDraft = undefined;
        }
        this.themeMessage = "";
        group.disabled = true;
        status.textContent = "";
        void this.configurationService.updateValue(
          WorkbenchConfiguration.colorTheme,
          descriptor.value,
        ).catch((error: unknown) => {
          status.textContent = error instanceof Error
            ? `Unable to save theme: ${error.message}`
            : "Unable to save theme.";
          const currentPreference = this.configurationService.getValue(
            WorkbenchConfiguration.colorTheme,
          );
          for (const candidate of options.querySelectorAll<HTMLInputElement>(
            "input[type='radio']",
          )) {
            candidate.checked = candidate.value === currentPreference;
          }
        }).finally(() => {
          group.disabled = false;
        });
      }));
      options.append(label);
    }
    const status = document.createElement("p");
    status.className = "zeta-theme-setting-status";
    status.setAttribute("role", "status");
    status.textContent = this.themeMessage;
    if (this.themeMessage) status.classList.add("is-success");
    const customization = document.createElement("div");
    customization.className = "zeta-theme-customization";
    const customize = document.createElement("button");
    customize.className = "zeta-theme-action";
    customize.type = "button";
    customize.disabled = !this.userThemeService.available;
    customize.textContent = this.activeUserThemeId() ? "Edit user theme JSON" : "Create from current theme";
    this.sectionBindings.add(addDisposableListener(customize, "click", () => this.startThemeEditing()));
    customization.append(customize);
    group.append(legend, hint, options, status, customization);
    const draft = this.themeDraft;
    if (draft) group.append(this.renderThemeEditor(document, group, status, draft));
    const userThemeStatus = renderUserThemeStatus(document, this.userThemeService);
    if (userThemeStatus) group.append(userThemeStatus);
    this.sectionContent.replaceChildren(group);
    this.contentScrollable.layout();
  }

  private activeUserThemeId(): string | undefined {
    const preference = this.configurationService.getValue(WorkbenchConfiguration.colorTheme);
    return preference === SystemColorThemePreference || !this.userThemeService.sourceFor(preference) ? undefined : preference;
  }

  private startThemeEditing(): void {
    const currentTheme = this.themeService.getColorTheme();
    const userThemeId = this.activeUserThemeId();
    const existingSource = userThemeId ? this.userThemeService.getSource(userThemeId) : undefined;
    if (userThemeId) {
      if (!existingSource) {
        this.themeMessage = `Unable to read the JSON source for '${userThemeId}'.`;
        this.renderAppearance();
        return;
      }
      this.themeDraft = { kind: "update", originalTheme: currentTheme, source: existingSource, themeId: userThemeId };
    } else {
      this.themeDraft = {
        kind: "create",
        originalTheme: currentTheme,
        source: serializeUserColorThemeDraft(currentTheme, this.availableDraftId(currentTheme), `My ${currentTheme.colorScheme === "light" ? "Light" : "Dark"} Theme`),
      };
    }
    this.themeMessage = "";
    this.renderAppearance();
    this.sectionContent.querySelector<HTMLTextAreaElement>(".zeta-theme-json-editor")?.focus();
  }

  private availableDraftId(theme: IColorTheme): string {
    const base = theme.colorScheme === "light" ? "my-light-theme" : "my-dark-theme";
    let candidate = base;
    let suffix = 2;
    while (WorkbenchThemesRegistry.getColorTheme(candidate)) candidate = `${base}-${suffix++}`;
    return candidate;
  }

  private renderThemeEditor(document: Document, group: HTMLFieldSetElement, status: HTMLParagraphElement, draft: ThemeDraft): HTMLElement {
    const editor = document.createElement("section");
    editor.className = "zeta-theme-json";
    const heading = document.createElement("h4");
    heading.textContent = draft.kind === "update" ? "Edit user theme JSON" : "New theme from current appearance";
    const hint = document.createElement("p");
    hint.textContent = draft.kind === "update"
      ? "Valid changes preview immediately. Save updates this user theme; change id and label before using Save As."
      : "This is a complete copy of the current Light or Dark theme. Change id, label, and colors, then save it as a new theme.";
    const textarea = document.createElement("textarea");
    textarea.className = "zeta-theme-json-editor";
    textarea.value = draft.source;
    textarea.spellcheck = false;
    textarea.setAttribute("aria-label", "User theme JSON");
    const actions = document.createElement("div");
    actions.className = "zeta-theme-json-actions";
    const preview = (): boolean => {
      draft.source = textarea.value;
      try {
        const theme = parseUserColorTheme(draft.source);
        this.themeService.setColorTheme(theme);
        status.textContent = `Previewing ${theme.label}.`;
        status.classList.add("is-success");
        return true;
      } catch (error) {
        status.textContent = error instanceof Error ? error.message : "Theme JSON is invalid.";
        status.classList.remove("is-success");
        return false;
      }
    };
    this.sectionBindings.add(addDisposableListener(textarea, "input", () => preview()));
    if (draft.kind === "update") {
      const save = themeAction(document, "Save");
      this.sectionBindings.add(addDisposableListener(save, "click", () => {
        if (preview()) void this.saveThemeDraft("save", group, status);
      }));
      actions.append(save);
    }
    const saveAs = themeAction(document, "Save As");
    this.sectionBindings.add(addDisposableListener(saveAs, "click", () => {
      if (preview()) void this.saveThemeDraft("saveAs", group, status);
    }));
    if (draft.kind === "update") {
      const remove = themeAction(document, "Delete");
      remove.classList.add("is-danger");
      this.sectionBindings.add(addDisposableListener(remove, "click", () => {
        void this.deleteThemeDraft(group, status);
      }));
      actions.append(remove);
    }
    const cancel = themeAction(document, "Cancel");
    this.sectionBindings.add(addDisposableListener(cancel, "click", () => this.cancelThemeEditing()));
    actions.append(saveAs, cancel);
    editor.append(heading, hint, textarea, actions);
    return editor;
  }

  private async saveThemeDraft(operation: "save" | "saveAs", group: HTMLFieldSetElement, status: HTMLParagraphElement): Promise<void> {
    const draft = this.themeDraft;
    if (!draft) return;
    group.disabled = true;
    status.classList.remove("is-success");
    status.textContent = operation === "save" ? "Saving theme…" : "Saving new theme…";
    try {
      const result = operation === "save"
        ? await this.userThemeService.save(draft.kind === "update" ? draft.themeId : "", draft.source)
        : await this.userThemeService.saveAs(draft.source);
      this.themeDraft = undefined;
      this.themeService.setColorTheme(result.theme);
      this.themeMessage = `Saved ${result.theme.label} to ${result.file}.`;
      await this.configurationService.updateValue(WorkbenchConfiguration.colorTheme, result.theme.id);
      this.renderAppearance();
    } catch (error) {
      status.textContent = error instanceof Error ? `Unable to save theme: ${error.message}` : "Unable to save theme.";
      group.disabled = false;
    }
  }

  private async deleteThemeDraft(group: HTMLFieldSetElement, status: HTMLParagraphElement): Promise<void> {
    const draft = this.themeDraft;
    if (!draft || draft.kind !== "update") return;
    const theme = WorkbenchThemesRegistry.getColorTheme(draft.themeId);
    if (!theme) {
      status.textContent = `User theme is not loaded: ${draft.themeId}`;
      return;
    }
    group.disabled = true;
    const confirmed = await this.dialogService.confirm({
      title: "Delete user theme?",
      message: `Delete “${theme.label}”?`,
      detail: `This permanently deletes ${this.userThemeService.sourceFor(theme.id)?.file ?? "the theme JSON file"} from the user theme folder.`,
      primaryButton: "Delete",
      cancelButton: "Cancel",
    });
    if (!confirmed) {
      group.disabled = false;
      return;
    }
    try {
      const result = await this.userThemeService.delete(draft.themeId);
      const fallback = isDarkColorScheme(result.colorScheme) ? darkColorTheme : lightColorTheme;
      this.themeDraft = undefined;
      this.themeService.setColorTheme(fallback);
      this.themeMessage = `Deleted ${theme.label} (${result.file}) and switched to ${fallback.label}.`;
      try {
        await this.configurationService.updateValue(WorkbenchConfiguration.colorTheme, fallback.id);
      } catch (error) {
        this.themeMessage = error instanceof Error
          ? `Deleted ${theme.label}, but could not save the fallback theme: ${error.message}`
          : `Deleted ${theme.label}, but could not save the fallback theme.`;
      }
      this.renderAppearance();
    } catch (error) {
      status.textContent = error instanceof Error ? `Unable to delete theme: ${error.message}` : "Unable to delete theme.";
      group.disabled = false;
    }
  }

  private handleNavigationKeydown(event: KeyboardEvent, sectionId: string): void {
    const visibleSections = this.visibleNavigationSections();
    const currentIndex = visibleSections.findIndex((section) => section.id === sectionId);
    let targetIndex: number | undefined;
    if (event.key === "ArrowUp") targetIndex = Math.max(0, currentIndex - 1);
    else if (event.key === "ArrowDown") targetIndex = Math.min(visibleSections.length - 1, currentIndex + 1);
    else if (event.key === "Home") targetIndex = 0;
    else if (event.key === "End") targetIndex = visibleSections.length - 1;
    if (targetIndex === undefined || targetIndex === currentIndex) return;
    stopEvent(event);
    this.navigationItems.get(visibleSections[targetIndex].id)?.focus();
  }

  private filterNavigation(value: string): void {
    const query = value.trim().toLocaleLowerCase();
    let matches = 0;
    for (const section of SettingsSections) {
      const visible = !query || `${section.label} ${section.description}`.toLocaleLowerCase().includes(query);
      const item = this.navigationItems.get(section.id)?.parentElement;
      if (item) item.hidden = !visible;
      if (visible) matches++;
    }
    this.navigationEmpty.hidden = matches !== 0;
    this.navigationScrollable.scrollTo(0, 0);
    this.navigationScrollable.layout();
  }

  private visibleNavigationSections(): readonly SettingsSectionDescriptor[] {
    return SettingsSections.filter((section) => {
      const item = this.navigationItems.get(section.id)?.parentElement;
      return item ? !item.hidden : false;
    });
  }

}

interface ThemeOptionDescriptor {
  readonly value: string;
  readonly label: string;
  readonly description: string;
  readonly previewThemes: readonly IColorTheme[];
}

type ThemeDraft =
  | { readonly kind: "create"; readonly originalTheme: IColorTheme; source: string }
  | { readonly kind: "update"; readonly originalTheme: IColorTheme; source: string; readonly themeId: string };

function themeOptions(userThemeService: IUserThemeService): readonly ThemeOptionDescriptor[] {
  return [
    {
      value: SystemColorThemePreference,
      label: "System",
      description: "Automatically follow the operating system.",
      previewThemes: [lightColorTheme, darkColorTheme],
    },
    ...WorkbenchThemesRegistry.getColorThemes().map((theme) => {
      const source = userThemeService.sourceFor(theme.id);
      return {
        value: theme.id,
        label: theme.label,
        description: source ? `User theme · ${source.file}` : `Use ${theme.label} on this device.`,
        previewThemes: [theme],
      };
    }),
  ];
}

function renderUserThemeStatus(document: Document, userThemeService: IUserThemeService): HTMLElement | undefined {
  if (!userThemeService.directory && userThemeService.issues.length === 0) return undefined;
  const container = document.createElement("div");
  container.className = "zeta-user-theme-status";
  if (userThemeService.directory) {
    const directory = document.createElement("p");
    directory.textContent = `User theme folder: ${userThemeService.directory}`;
    container.append(directory);
  }
  if (userThemeService.issues.length > 0) {
    const heading = document.createElement("p");
    heading.textContent = "Some user themes could not be loaded:";
    const list = document.createElement("ul");
    for (const issue of userThemeService.issues) {
      const item = document.createElement("li");
      item.textContent = `${issue.file}: ${issue.message}`;
      list.append(item);
    }
    container.append(heading, list);
  }
  const restart = document.createElement("p");
  restart.textContent = "Themes saved here are available immediately. Restart Zeta after external file changes.";
  container.append(restart);
  return container;
}

function themeAction(document: Document, label: string): HTMLButtonElement {
  const button = document.createElement("button");
  button.className = "zeta-theme-action";
  button.type = "button";
  button.textContent = label;
  return button;
}

function applyThemePreview(preview: HTMLElement, themes: readonly IColorTheme[]): void {
  const values = (id: string) => themes.map((theme) => requiredThemeColor(theme, id));
  preview.style.setProperty("--theme-preview-editor", previewValue(values(ColorId.editorBackground)));
  preview.style.setProperty("--theme-preview-sidebar", previewValue(values(ColorId.sideBarBackground)));
  preview.style.setProperty("--theme-preview-control", previewValue(values(ColorId.inputBackground)));
}

function requiredThemeColor(theme: IColorTheme, id: string): string {
  const value = theme.getColorCss(id);
  if (!value) throw new Error(`Theme '${theme.id}' does not define preview color '${id}'`);
  return value;
}

function previewValue(values: readonly string[]): string {
  if (values.length === 1) return values[0];
  if (values.length === 2) return `linear-gradient(135deg, ${values[0]} 0 50%, ${values[1]} 50%)`;
  throw new Error("Theme previews support one concrete theme or a light/dark pair");
}
