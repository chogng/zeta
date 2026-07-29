import "./media/settingsEditor.css";
import { addDisposableListener, stopEvent } from "../../../../base/browser/dom.js";
import { InputBox } from "../../../../base/browser/ui/inputbox/inputbox.js";
import { ScrollableElement } from "../../../../base/browser/ui/scrollbar/scrollableElement.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import type { ISettingsService } from "../../../services/preferences/common/settings.js";
import { getSettingsSection, SettingsSections, type SettingsSectionDescriptor } from "../common/settingsSections.js";

export interface SettingsEditorOptions {
  readonly ownerDocument: Document;
  readonly settingsService: ISettingsService;
}

let nextSettingsEditorId = 1;

/** Search, navigation, and page content hosted by the Workbench modal editor. */
export class SettingsEditor extends DisposableOwner {
  readonly element: HTMLDivElement;
  readonly #settingsService: ISettingsService;
  readonly #searchInput: InputBox;
  readonly #navigationItems = new Map<string, HTMLButtonElement>();
  readonly #navigationEmpty: HTMLParagraphElement;
  readonly #navigationScrollable: ScrollableElement;
  readonly #contentScrollable: ScrollableElement;
  readonly #content: HTMLElement;
  readonly #contentHeading: HTMLHeadingElement;
  readonly #contentDescription: HTMLParagraphElement;

  constructor(options: SettingsEditorOptions) {
    super();
    this.#settingsService = options.settingsService;
    const editorId = `zeta-settings-editor-${nextSettingsEditorId++}`;
    this.element = options.ownerDocument.createElement("div");
    this.element.className = "zeta-settings-editor";

    const search = options.ownerDocument.createElement("div");
    search.className = "zeta-settings-search";
    search.setAttribute("role", "search");
    this.#searchInput = this.own(new InputBox({
      ownerDocument: options.ownerDocument,
      type: "search",
      placeholder: "Search settings",
      ariaLabel: "Search settings",
      ariaControls: `${editorId}-navigation`,
    }));
    this.#searchInput.element.classList.add("zeta-settings-search-input");
    search.append(this.#searchInput.element);

    const layout = options.ownerDocument.createElement("div");
    layout.className = "zeta-settings-layout";

    const navigation = options.ownerDocument.createElement("nav");
    navigation.className = "zeta-settings-sidebar";
    navigation.setAttribute("aria-label", "Settings categories");
    this.#navigationScrollable = this.own(new ScrollableElement({
      ownerDocument: options.ownerDocument,
      direction: "vertical",
      vertical: "auto",
      tabIndex: -1,
      wheel: { consume: "when-scrolling" },
    }));
    this.#navigationScrollable.element.classList.add("zeta-settings-sidebar-scrollable");
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
      this.#navigationItems.set(section.id, button);
      this.own(addDisposableListener(button, "click", () => {
        this.#settingsService.open(section.id);
      }));
      this.own(addDisposableListener(button, "keydown", (event: KeyboardEvent) => {
        this.#handleNavigationKeydown(event, section.id);
      }));
      item.append(button);
      navigationList.append(item);
    }
    this.#navigationEmpty = options.ownerDocument.createElement("p");
    this.#navigationEmpty.className = "zeta-settings-navigation-empty";
    this.#navigationEmpty.textContent = "No settings found.";
    this.#navigationEmpty.setAttribute("role", "status");
    this.#navigationEmpty.hidden = true;
    this.#navigationScrollable.append(navigationList, this.#navigationEmpty);
    navigation.append(this.#navigationScrollable.element);

    this.#content = options.ownerDocument.createElement("main");
    this.#content.className = "zeta-settings-page";
    this.#content.dataset.settingsContainer = "";
    this.#content.tabIndex = -1;
    this.#contentScrollable = this.own(new ScrollableElement({
      ownerDocument: options.ownerDocument,
      direction: "vertical",
      vertical: "auto",
      tabIndex: -1,
      wheel: { consume: "when-scrolling" },
    }));
    this.#contentScrollable.element.classList.add("zeta-settings-page-scrollable");
    const contentInner = options.ownerDocument.createElement("div");
    contentInner.className = "zeta-settings-page-inner";
    this.#contentHeading = options.ownerDocument.createElement("h3");
    this.#contentHeading.id = `${editorId}-section`;
    this.#content.setAttribute("aria-labelledby", this.#contentHeading.id);
    this.#contentDescription = options.ownerDocument.createElement("p");
    this.#contentDescription.className = "zeta-settings-description";
    const placeholder = options.ownerDocument.createElement("div");
    placeholder.className = "zeta-settings-section-content";
    placeholder.dataset.settingsSectionContent = "";
    contentInner.append(this.#contentHeading, this.#contentDescription, placeholder);
    this.#contentScrollable.append(contentInner);
    this.#content.append(this.#contentScrollable.element);

    layout.append(navigation, this.#content);
    this.element.append(search, layout);
    this.#renderSection(getSettingsSection(this.#settingsService.activeSectionId));

    this.own(this.#settingsService.onDidChangeActiveSection((sectionId) => {
      this.#renderSection(getSettingsSection(sectionId));
    }));
    this.own(this.#searchInput.onDidChange((value) => {
      this.#filterNavigation(value);
    }));
    this.own(this.#searchInput.onKeyDown((event) => {
      if (event.key === "Escape" && this.#searchInput.value) {
        stopEvent(event);
        this.#searchInput.value = "";
        return;
      }
      if (event.key !== "ArrowDown") return;
      const firstVisible = this.#visibleNavigationSections()[0];
      if (!firstVisible) return;
      stopEvent(event);
      this.#navigationItems.get(firstVisible.id)?.focus();
    }));
    this.defer(() => this.element.remove());
  }

  focus(): void {
    this.#searchInput.focus();
  }

  layout(): void {
    this.#navigationScrollable.layout();
    this.#contentScrollable.layout();
  }

  #renderSection(section: SettingsSectionDescriptor): void {
    for (const [sectionId, item] of this.#navigationItems) {
      const active = sectionId === section.id;
      item.classList.toggle("is-active", active);
      if (active) item.setAttribute("aria-current", "page");
      else item.removeAttribute("aria-current");
    }
    this.#content.dataset.activeSettingsSection = section.id;
    this.#contentHeading.textContent = section.label;
    this.#contentDescription.textContent = section.description;
    this.#contentScrollable.scrollTo(0, 0);
    this.#contentScrollable.layout();
  }

  #handleNavigationKeydown(event: KeyboardEvent, sectionId: string): void {
    const visibleSections = this.#visibleNavigationSections();
    const currentIndex = visibleSections.findIndex((section) => section.id === sectionId);
    let targetIndex: number | undefined;
    if (event.key === "ArrowUp") targetIndex = Math.max(0, currentIndex - 1);
    else if (event.key === "ArrowDown") targetIndex = Math.min(visibleSections.length - 1, currentIndex + 1);
    else if (event.key === "Home") targetIndex = 0;
    else if (event.key === "End") targetIndex = visibleSections.length - 1;
    if (targetIndex === undefined || targetIndex === currentIndex) return;
    stopEvent(event);
    this.#navigationItems.get(visibleSections[targetIndex].id)?.focus();
  }

  #filterNavigation(value: string): void {
    const query = value.trim().toLocaleLowerCase();
    let matches = 0;
    for (const section of SettingsSections) {
      const visible = !query || `${section.label} ${section.description}`.toLocaleLowerCase().includes(query);
      const item = this.#navigationItems.get(section.id)?.parentElement;
      if (item) item.hidden = !visible;
      if (visible) matches++;
    }
    this.#navigationEmpty.hidden = matches !== 0;
    this.#navigationScrollable.scrollTo(0, 0);
    this.#navigationScrollable.layout();
  }

  #visibleNavigationSections(): readonly SettingsSectionDescriptor[] {
    return SettingsSections.filter((section) => {
      const item = this.#navigationItems.get(section.id)?.parentElement;
      return item ? !item.hidden : false;
    });
  }

}
