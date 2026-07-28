import { Emitter } from "../../../common/event.js";
import {
  DisposableOwner,
} from "../../../common/lifecycle.js";
import {
  addDisposableListener,
  isHTMLElement,
  stopEvent,
} from "../../dom.js";
import { focusPreservingScroll } from "../../focus.js";
import {
  Dropdown,
} from "../dropdown/dropdown.js";
import type {
  IContextViewProvider,
} from "../contextview/contextview.js";

export interface SelectOption {
  readonly value: string;
  readonly label: string;
  readonly description?: string;
  readonly disabled?: boolean;
}

export interface SelectBoxOptions {
  readonly options: readonly SelectOption[];
  readonly selectedValue?: string;
  readonly ownerDocument?: Document;
  readonly ariaLabel?: string;
  readonly contextViewProvider?: IContextViewProvider;
}

export interface SelectBoxSelection {
  readonly value: string;
  readonly index: number;
  readonly option: SelectOption;
}

let selectBoxId = 0;

/** A keyboard-accessible custom select backed by an anchored Dropdown. */
export class SelectBox extends DisposableOwner {
  readonly element: HTMLDivElement;
  readonly #dropdown: Dropdown;
  readonly #list: HTMLDivElement;
  readonly #onDidSelect = this.own(new Emitter<SelectBoxSelection>());
  readonly onDidSelect = this.#onDidSelect.event;
  #options: readonly SelectOption[] = [];
  #selectedIndex = -1;
  #activeIndex = -1;
  #enabled = true;

  constructor(options: SelectBoxOptions) {
    super();
    const ownerDocument = options.ownerDocument ?? document;
    const list = ownerDocument.createElement("div");
    this.#list = list;
    selectBoxId += 1;
    list.id = `zeta-select-box-list-${selectBoxId}`;
    list.className = "zeta-select-box-list";
    list.setAttribute("role", "listbox");
    list.tabIndex = -1;

    const dropdown = this.own(new Dropdown({
      label: "",
      content: list,
      ownerDocument,
      ariaLabel: options.ariaLabel,
      gap: 2,
      contextViewProvider: options.contextViewProvider,
    }));
    this.#dropdown = dropdown;
    this.element = dropdown.element;
    this.element.classList.add("zeta-select-box");
    dropdown.button.classList.add("zeta-select-box-button");
    dropdown.button.setAttribute("role", "combobox");
    dropdown.button.setAttribute("aria-haspopup", "listbox");
    dropdown.button.setAttribute("aria-controls", list.id);
    dropdown.button.setAttribute("aria-autocomplete", "none");

    this.own(dropdown.onDidChangeVisibility(({ visible }) => {
      if (!visible) return;
      this.#setActiveIndex(this.#selectedIndex);
      this.#focusActiveOption();
    }));
    this.own(addDisposableListener(list, "click", (event) => {
      const index = this.#optionIndexFromTarget(event.target);
      if (index === undefined) return;
      this.#commitSelection(index);
    }));
    this.own(addDisposableListener(list, "pointermove", (event) => {
      const index = this.#optionIndexFromTarget(event.target);
      if (index === undefined || this.#options[index]?.disabled) return;
      this.#setActiveIndex(index);
    }));
    this.own(addDisposableListener(list, "keydown", (event) => {
      this.#onListKeyDown(event);
    }));
    this.own(addDisposableListener(dropdown.button, "keydown", (event) => {
      if (event.key === "Home" || event.key === "End") {
        stopEvent(event);
        dropdown.show();
        this.#moveActive(event.key === "Home" ? "first" : "last");
      }
    }));

    this.setOptions(options.options);
    if (options.selectedValue !== undefined) {
      this.value = options.selectedValue;
    }
  }

  get options(): readonly SelectOption[] {
    return this.#options;
  }

  get selectedIndex(): number {
    return this.#selectedIndex;
  }

  get value(): string | undefined {
    return this.#options[this.#selectedIndex]?.value;
  }

  set value(value: string | undefined) {
    if (value === undefined) {
      this.#applySelection(-1);
      return;
    }
    const index = this.#options.findIndex((option) =>
      option.value === value
    );
    if (index < 0) {
      throw new RangeError(`Unknown select option '${value}'`);
    }
    if (this.#options[index]?.disabled) {
      throw new RangeError(`Select option '${value}' is disabled`);
    }
    this.#applySelection(index);
  }

  get enabled(): boolean {
    return this.#enabled;
  }

  set enabled(value: boolean) {
    this.#enabled = value;
    this.#syncEnabledState();
  }

  setOptions(options: readonly SelectOption[]): void {
    validateOptions(options);
    const previousValue = this.value;
    this.#options = [...options];
    this.#renderOptions();
    const preservedIndex = previousValue === undefined
      ? -1
      : this.#options.findIndex((option) =>
        option.value === previousValue && !option.disabled
      );
    this.#applySelection(
      preservedIndex >= 0 ? preservedIndex : this.#firstEnabledIndex(),
    );
    this.#syncEnabledState();
  }

  focus(): void {
    this.#dropdown.focus();
  }

  setAriaLabel(label: string): void {
    this.#dropdown.button.setAttribute("aria-label", label);
  }

  blur(): void {
    this.#dropdown.button.blur();
  }

  show(): void {
    this.#dropdown.show();
  }

  hide(): void {
    this.#dropdown.hide();
  }

  #renderOptions(): void {
    const ownerDocument = this.#list.ownerDocument;
    const elements = this.#options.map((option, index) => {
      const element = ownerDocument.createElement("div");
      element.id = `${this.#list.id}-option-${index}`;
      element.className = "zeta-select-box-option";
      element.dataset.index = String(index);
      element.setAttribute("role", "option");
      element.setAttribute("aria-selected", "false");
      element.tabIndex = -1;
      if (option.disabled) {
        element.classList.add("zeta-select-box-option-disabled");
        element.setAttribute("aria-disabled", "true");
      }
      const label = ownerDocument.createElement("span");
      label.className = "zeta-select-box-option-label";
      label.textContent = option.label;
      element.append(label);
      if (option.description) {
        const description = ownerDocument.createElement("span");
        description.className = "zeta-select-box-option-description";
        description.textContent = option.description;
        element.append(description);
      }
      return element;
    });
    this.#list.replaceChildren(...elements);
  }

  #applySelection(index: number): void {
    this.#selectedIndex = index;
    this.#setActiveIndex(index);
    const option = this.#options[index];
    this.#dropdown.setLabel(option?.label ?? "");
    this.#dropdown.button.title = option?.label ?? "";
    for (const [optionIndex, element] of this.#optionElements().entries()) {
      const selected = optionIndex === index;
      element.classList.toggle("zeta-select-box-option-selected", selected);
      element.setAttribute("aria-selected", String(selected));
    }
  }

  #commitSelection(index: number): void {
    const option = this.#options[index];
    if (!option || option.disabled) return;
    const changed = index !== this.#selectedIndex;
    this.#applySelection(index);
    this.#dropdown.hide();
    if (!changed) return;
    this.#onDidSelect.fire({
      value: option.value,
      index,
      option,
    });
  }

  #onListKeyDown(event: KeyboardEvent): void {
    switch (event.key) {
      case "ArrowDown":
        stopEvent(event);
        this.#moveActive("next");
        break;
      case "ArrowUp":
        stopEvent(event);
        this.#moveActive("previous");
        break;
      case "Home":
        stopEvent(event);
        this.#moveActive("first");
        break;
      case "End":
        stopEvent(event);
        this.#moveActive("last");
        break;
      case "Enter":
      case " ":
        stopEvent(event);
        this.#commitSelection(this.#activeIndex);
        break;
      case "Tab":
        this.#dropdown.hide();
        break;
      default:
        if (
          !event.isComposing &&
          event.key.length === 1 &&
          !event.altKey &&
          !event.ctrlKey &&
          !event.metaKey
        ) {
          this.#moveActiveByPrefix(event.key);
        }
    }
  }

  #moveActiveByPrefix(prefix: string): void {
    const normalized = prefix.toLocaleLowerCase();
    const start = Math.max(0, this.#activeIndex + 1);
    for (let offset = 0; offset < this.#options.length; offset += 1) {
      const index = (start + offset) % this.#options.length;
      const option = this.#options[index];
      if (
        option &&
        !option.disabled &&
        option.label.toLocaleLowerCase().startsWith(normalized)
      ) {
        this.#setActiveIndex(index);
        this.#focusActiveOption();
        return;
      }
    }
  }

  #moveActive(
    direction: "next" | "previous" | "first" | "last",
  ): void {
    if (this.#options.length === 0) return;
    let index: number;
    let step: number;
    if (direction === "first") {
      index = 0;
      step = 1;
    } else if (direction === "last") {
      index = this.#options.length - 1;
      step = -1;
    } else {
      step = direction === "next" ? 1 : -1;
      index = this.#activeIndex < 0
        ? (step > 0 ? 0 : this.#options.length - 1)
        : this.#activeIndex + step;
    }
    while (index >= 0 && index < this.#options.length) {
      if (!this.#options[index]?.disabled) {
        this.#setActiveIndex(index);
        this.#focusActiveOption();
        return;
      }
      index += step;
    }
  }

  #setActiveIndex(index: number): void {
    this.#activeIndex = index;
    if (index < 0) {
      this.#dropdown.button.removeAttribute("aria-activedescendant");
    } else {
      this.#dropdown.button.setAttribute(
        "aria-activedescendant",
        `${this.#list.id}-option-${index}`,
      );
    }
    for (const [optionIndex, element] of this.#optionElements().entries()) {
      element.classList.toggle(
        "zeta-select-box-option-active",
        optionIndex === index,
      );
    }
  }

  #focusActiveOption(): void {
    const option = this.#optionElements()[this.#activeIndex];
    if (option) focusPreservingScroll(option);
    option?.scrollIntoView({
      block: "nearest",
    });
  }

  #optionElements(): HTMLElement[] {
    return Array.from(
      this.#list.querySelectorAll<HTMLElement>(
        ".zeta-select-box-option",
      ),
    );
  }

  #optionIndexFromTarget(target: EventTarget | null): number | undefined {
    if (!isHTMLElement(target)) return undefined;
    const option = target.closest<HTMLElement>(".zeta-select-box-option");
    if (!option || !this.#list.contains(option)) return undefined;
    const index = Number.parseInt(option.dataset.index ?? "", 10);
    return Number.isInteger(index) ? index : undefined;
  }

  #firstEnabledIndex(): number {
    return this.#options.findIndex((option) => !option.disabled);
  }

  #syncEnabledState(): void {
    this.#dropdown.enabled = this.#enabled && this.#firstEnabledIndex() >= 0;
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
