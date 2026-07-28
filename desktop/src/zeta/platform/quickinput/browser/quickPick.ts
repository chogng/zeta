import { stopEvent } from "../../../base/browser/dom.js";
import {
  setAriaAttribute,
  setRole,
} from "../../../base/browser/ui/aria/aria.js";
import {
  InputBox,
} from "../../../base/browser/ui/inputbox/inputbox.js";
import { Emitter, type Event } from "../../../base/common/event.js";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
import type {
  IQuickPick,
  IQuickPickItem,
} from "../common/quickInput.js";
import { QuickInputList } from "./quickInputList.js";

export interface BrowserQuickPickOptions {
  readonly ownerDocument: Document;
  readonly onShow: (quickPick: IBrowserQuickPickHost) => void;
  readonly onHide: (quickPick: IBrowserQuickPickHost) => void;
  readonly onDispose: (quickPick: IBrowserQuickPickHost) => void;
}

/** Narrow controller used by the shared Workbench host. */
export interface IBrowserQuickPickHost {
  readonly element: HTMLDivElement;
  focus(): void;
  hide(): void;
  dispose(): void;
}

/** DOM implementation of one searchable Quick Pick controller. */
export class BrowserQuickPick<TItem extends IQuickPickItem>
  extends DisposableOwner
  implements IQuickPick<TItem> {
  readonly element: HTMLDivElement;
  readonly #inputBox: InputBox;
  readonly #list: QuickInputList<TItem>;
  readonly #onDidAccept = this.own(new Emitter<TItem>());
  readonly #onDidChangeValue = this.own(new Emitter<string>());
  readonly #onDidHide = this.own(new Emitter<void>());
  readonly #options: BrowserQuickPickOptions;
  #visible = false;
  #placeholder = "";

  readonly onDidAccept: Event<TItem> = this.#onDidAccept.event;
  readonly onDidChangeValue: Event<string> =
    this.#onDidChangeValue.event;
  readonly onDidHide: Event<void> = this.#onDidHide.event;

  constructor(options: BrowserQuickPickOptions) {
    super();
    this.#options = options;
    const ownerDocument = options.ownerDocument;
    this.element = ownerDocument.createElement("div");
    this.element.className = "zeta-quick-pick";
    setRole(this.element, "dialog");
    setAriaAttribute(this.element, "label", "Quick Pick");
    this.defer(() => {
      if (this.#visible) this.hide();
      options.onDispose(this);
      this.element.remove();
    });

    this.#list = this.own(new QuickInputList<TItem>(ownerDocument));
    this.#inputBox = this.own(new InputBox({
      ownerDocument,
      type: "search",
      ariaLabel: "Quick Pick",
      role: "combobox",
      ariaAutoComplete: "list",
      ariaControls: this.#list.listId,
      ariaExpanded: true,
    }));
    this.#inputBox.element.classList.add("zeta-quick-pick-input");
    this.element.append(
      this.#inputBox.element,
      this.#list.element,
    );

    this.own(this.#inputBox.onDidChange(
      (value) => this.#handleValueChange(value),
    ));
    this.own(this.#list.onDidAccept((item) => {
      this.#onDidAccept.fire(item);
    }));
    this.own(this.#list.onDidChangeActive(({ rowId }) => {
      this.#inputBox.ariaActiveDescendant = rowId;
    }));
    this.own(this.#inputBox.onKeyDown(
      (event: KeyboardEvent) => this.#handleKeyDown(event),
    ));
  }

  get items(): readonly TItem[] {
    return this.#list.items;
  }

  set items(items: readonly TItem[]) {
    this.#list.items = items;
  }

  get placeholder(): string {
    return this.#placeholder;
  }

  set placeholder(value: string) {
    this.#placeholder = value;
    this.#inputBox.placeholder = value;
  }

  get value(): string {
    return this.#inputBox.value;
  }

  set value(value: string) {
    this.#inputBox.value = value;
  }

  show(): void {
    if (this.#visible) {
      this.focus();
      return;
    }
    this.#visible = true;
    this.#options.onShow(this);
    this.focus();
  }

  hide(): void {
    if (!this.#visible) return;
    this.#visible = false;
    this.#options.onHide(this);
    this.#onDidHide.fire();
  }

  focus(): void {
    this.#inputBox.focus();
    this.#inputBox.select();
  }

  #handleValueChange(value: string): void {
    this.#list.filter(value);
    this.#onDidChangeValue.fire(value);
  }

  #handleKeyDown(event: KeyboardEvent): void {
    switch (event.key) {
      case "ArrowDown":
        stopEvent(event);
        this.#list.focusNext();
        break;
      case "ArrowUp":
        stopEvent(event);
        this.#list.focusPrevious();
        break;
      case "Enter":
        stopEvent(event);
        this.#list.acceptActive();
        break;
      case "Escape":
        stopEvent(event);
        this.hide();
        break;
    }
  }

}
