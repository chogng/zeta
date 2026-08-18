import { Emitter } from "../../../common/event.js";
import { DisposableOwner } from "../../../common/lifecycle.js";
import { lxiconsLibrary } from "../../../common/lxiconsLibrary.js";
import { addDisposableListener, isHTMLElement, stopEvent, h } from "../../dom.js";
import { focusPreservingScroll } from "../../focus.js";
import { setAriaAttribute, setRole } from "../aria/aria.js";
import type { IContextViewProvider } from "../contextview/contextview.js";
import { Dropdown } from "../dropdown/dropdown.js";
import { appendIcon } from "../icon/icon.js";

export interface SelectOption {
  readonly value: string;
  readonly label: string;
  readonly description?: string;
  readonly disabled?: boolean;
}

export interface SelectBoxOptions {
  readonly options: readonly SelectOption[];
  readonly selectedValue?: string;
  readonly ariaLabel?: string;
  readonly presentation?: SelectBoxPresentation;
  readonly contextViewProvider?: IContextViewProvider;
}

/** Visual treatment for the select trigger while retaining the same behavior. */
export type SelectBoxPresentation = "default" | "field";

export interface SelectBoxSelection {
  readonly value: string;
  readonly index: number;
  readonly option: SelectOption;
}

let selectBoxId = 0;

/** A keyboard-accessible custom select backed by an anchored Dropdown. */
export class SelectBox extends DisposableOwner {
  readonly element: HTMLDivElement;
  private readonly dropdown: Dropdown;
  private readonly list: HTMLDivElement;
  private readonly _onDidSelect = this.own(new Emitter<SelectBoxSelection>());
  readonly onDidSelect = this._onDidSelect.event;
  private _options: readonly SelectOption[] = [];
  private _selectedIndex = -1;
  private activeIndex = -1;
  private _enabled = true;

  constructor(container: HTMLElement, options: SelectBoxOptions) {
    super();
    const ownerDocument = container.ownerDocument;
    const list = h(ownerDocument, "div");
    this.list = list;
    selectBoxId += 1;
    list.id = `zeta-select-box-list-${selectBoxId}`;
    list.className = "zeta-select-box-list";
    list.setAttribute("role", "listbox");
    list.tabIndex = -1;

    const dropdown = this.own(new Dropdown(container, {
      label: "",
      content: list,
      ariaLabel: options.ariaLabel,
      gap: 2,
      indicator: lxiconsLibrary.unfold,
      contentWidth: "at-least-trigger",
      contextViewProvider: options.contextViewProvider,
    }));
    this.dropdown = dropdown;
    this.element = dropdown.element;
    const presentation = options.presentation ?? "default";
    this.element.classList.add("zeta-select-box", `zeta-select-box-${presentation}`);
    dropdown.button.classList.add("zeta-select-box-button");
    setRole(dropdown.button, "combobox");
    setAriaAttribute(dropdown.button, "haspopup", "listbox");
    setAriaAttribute(dropdown.button, "controls", list.id);
    setAriaAttribute(dropdown.button, "autocomplete", "none");

    this.own(dropdown.onDidChangeVisibility(({ visible }) => {
      if (!visible) return;
      this.setActiveIndex(this._selectedIndex);
      this.focusActiveOption();
    }));
    this.own(addDisposableListener(list, "click", (event) => {
      const index = this.optionIndexFromTarget(event.target);
      if (index === undefined) return;
      this.commitSelection(index);
    }));
    this.own(addDisposableListener(list, "pointermove", (event) => {
      const index = this.optionIndexFromTarget(event.target);
      if (index === undefined || this._options[index]?.disabled) return;
      this.setActiveIndex(index);
    }));
    this.own(addDisposableListener(list, "keydown", (event) => {
      this.onListKeyDown(event);
    }));
    this.own(addDisposableListener(dropdown.button, "keydown", (event) => {
      if (event.key === "Home" || event.key === "End") {
        stopEvent(event);
        dropdown.show();
        this.moveActive(event.key === "Home" ? "first" : "last");
      }
    }));

    this.setOptions(options.options);
    if (options.selectedValue !== undefined) {
      this.value = options.selectedValue;
    }
  }

  get options(): readonly SelectOption[] {
    return this._options;
  }

  get selectedIndex(): number {
    return this._selectedIndex;
  }

  get value(): string | undefined {
    return this._options[this._selectedIndex]?.value;
  }

  set value(value: string | undefined) {
    if (value === undefined) {
      this.applySelection(-1);
      return;
    }
    const index = this._options.findIndex((option) =>
      option.value === value
    );
    if (index < 0) {
      throw new RangeError(`Unknown select option '${value}'`);
    }
    if (this._options[index]?.disabled) {
      throw new RangeError(`Select option '${value}' is disabled`);
    }
    this.applySelection(index);
  }

  get enabled(): boolean {
    return this._enabled;
  }

  set enabled(value: boolean) {
    this._enabled = value;
    this.syncEnabledState();
  }

  setOptions(options: readonly SelectOption[]): void {
    validateOptions(options);
    const previousValue = this.value;
    this._options = [...options];
    this.renderOptions();
    const preservedIndex = previousValue === undefined
      ? -1
      : this._options.findIndex((option) =>
        option.value === previousValue && !option.disabled
      );
    this.applySelection(
      preservedIndex >= 0 ? preservedIndex : this.firstEnabledIndex(),
    );
    this.syncEnabledState();
  }

  focus(): void {
    this.dropdown.focus();
  }

  setAriaLabel(label: string): void {
    setAriaAttribute(this.dropdown.button, "label", label);
  }

  blur(): void {
    this.dropdown.button.blur();
  }

  show(): void {
    this.dropdown.show();
  }

  hide(): void {
    this.dropdown.hide();
  }

  private renderOptions(): void {
    const ownerDocument = this.list.ownerDocument;
    const elements = this._options.map((option, index) => {
      const element = h(ownerDocument, "div");
      element.id = `${this.list.id}-option-${index}`;
      element.className = "zeta-select-box-option";
      element.dataset.index = String(index);
      setRole(element, "option");
      setAriaAttribute(element, "selected", false);
      element.tabIndex = -1;
      if (option.disabled) {
        element.classList.add("zeta-select-box-option-disabled");
        setAriaAttribute(element, "disabled", true);
      }
      const label = h(ownerDocument, "span");
      label.className = "zeta-select-box-option-label";
      label.textContent = option.label;
      element.append(label);
      if (option.description) {
        const description = h(ownerDocument, "span");
        description.className = "zeta-select-box-option-description";
        description.textContent = option.description;
        element.append(description);
      }
      const check = h(ownerDocument, "span");
      check.className = "zeta-select-box-option-check";
      setAriaAttribute(check, "hidden", true);
      appendIcon(lxiconsLibrary.check, check);
      element.append(check);
      return element;
    });
    this.list.replaceChildren(...elements);
  }

  private applySelection(index: number): void {
    this._selectedIndex = index;
    this.setActiveIndex(index);
    const option = this._options[index];
    this.dropdown.setLabel(option?.label ?? "");
    this.dropdown.button.title = option?.label ?? "";
    for (const [optionIndex, element] of this.optionElements().entries()) {
      const selected = optionIndex === index;
      element.classList.toggle("zeta-select-box-option-selected", selected);
      setAriaAttribute(element, "selected", selected);
    }
  }

  private commitSelection(index: number): void {
    const option = this._options[index];
    if (!option || option.disabled) return;
    const changed = index !== this._selectedIndex;
    this.applySelection(index);
    this.dropdown.hide();
    if (!changed) return;
    this._onDidSelect.fire({
      value: option.value,
      index,
      option,
    });
  }

  private onListKeyDown(event: KeyboardEvent): void {
    switch (event.key) {
      case "ArrowDown":
        stopEvent(event);
        this.moveActive("next");
        break;
      case "ArrowUp":
        stopEvent(event);
        this.moveActive("previous");
        break;
      case "Home":
        stopEvent(event);
        this.moveActive("first");
        break;
      case "End":
        stopEvent(event);
        this.moveActive("last");
        break;
      case "Enter":
      case " ":
        stopEvent(event);
        this.commitSelection(this.activeIndex);
        break;
      case "Tab":
        this.dropdown.hide();
        break;
      default:
        if (
          !event.isComposing &&
          event.key.length === 1 &&
          !event.altKey &&
          !event.ctrlKey &&
          !event.metaKey
        ) {
          this.moveActiveByPrefix(event.key);
        }
    }
  }

  private moveActiveByPrefix(prefix: string): void {
    const normalized = prefix.toLocaleLowerCase();
    const start = Math.max(0, this.activeIndex + 1);
    for (let offset = 0; offset < this._options.length; offset += 1) {
      const index = (start + offset) % this._options.length;
      const option = this._options[index];
      if (
        option &&
        !option.disabled &&
        option.label.toLocaleLowerCase().startsWith(normalized)
      ) {
        this.setActiveIndex(index);
        this.focusActiveOption();
        return;
      }
    }
  }

  private moveActive(
    direction: "next" | "previous" | "first" | "last",
  ): void {
    if (this._options.length === 0) return;
    let index: number;
    let step: number;
    if (direction === "first") {
      index = 0;
      step = 1;
    } else if (direction === "last") {
      index = this._options.length - 1;
      step = -1;
    } else {
      step = direction === "next" ? 1 : -1;
      index = this.activeIndex < 0
        ? (step > 0 ? 0 : this._options.length - 1)
        : this.activeIndex + step;
    }
    while (index >= 0 && index < this._options.length) {
      if (!this._options[index]?.disabled) {
        this.setActiveIndex(index);
        this.focusActiveOption();
        return;
      }
      index += step;
    }
  }

  private setActiveIndex(index: number): void {
    this.activeIndex = index;
    if (index < 0) {
      setAriaAttribute(
        this.dropdown.button,
        "activedescendant",
        undefined,
      );
    } else {
      setAriaAttribute(
        this.dropdown.button,
        "activedescendant",
        `${this.list.id}-option-${index}`,
      );
    }
    for (const [optionIndex, element] of this.optionElements().entries()) {
      element.classList.toggle(
        "zeta-select-box-option-active",
        optionIndex === index,
      );
    }
  }

  private focusActiveOption(): void {
    const option = this.optionElements()[this.activeIndex];
    if (option) focusPreservingScroll(option);
    option?.scrollIntoView({
      block: "nearest",
    });
  }

  private optionElements(): HTMLElement[] {
    return Array.from(
      this.list.querySelectorAll<HTMLElement>(
        ".zeta-select-box-option",
      ),
    );
  }

  private optionIndexFromTarget(target: EventTarget | null): number | undefined {
    if (!isHTMLElement(target)) return undefined;
    const option = target.closest<HTMLElement>(".zeta-select-box-option");
    if (!option || !this.list.contains(option)) return undefined;
    const index = Number.parseInt(option.dataset.index ?? "", 10);
    return Number.isInteger(index) ? index : undefined;
  }

  private firstEnabledIndex(): number {
    return this._options.findIndex((option) => !option.disabled);
  }

  private syncEnabledState(): void {
    this.dropdown.enabled = this._enabled && this.firstEnabledIndex() >= 0;
  }
}

function validateOptions(options: readonly SelectOption[]): void {
  const values = new Set<string>();
  for (const option of options) {
    if (values.has(option.value)) {
      throw new TypeError(`Duplicate select option value '${option.value}'`);
    }
    values.add(option.value);
  }
}
