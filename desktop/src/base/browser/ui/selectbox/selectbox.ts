import { Component } from "../common/component.js";

export interface SelectOption { value: string; label: string; disabled?: boolean; }

/** A typed wrapper around a native select element. */
export class SelectBox extends Component<HTMLSelectElement> {
  constructor(options: readonly SelectOption[], onChange?: (value: string) => void) {
    const element = document.createElement("select");
    element.className = "zeta-select-box";
    super(element);
    this.setOptions(options);
    if (onChange) this.listen(element, "change", () => onChange(element.value));
  }

  setOptions(options: readonly SelectOption[]): void {
    this.element.replaceChildren(...options.map((option) => {
      const item = new Option(option.label, option.value);
      item.disabled = option.disabled ?? false;
      return item;
    }));
  }

  get value(): string { return this.element.value; }
  set value(value: string) { this.element.value = value; }
}
