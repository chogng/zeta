import { WorkbenchPart } from "../../part.js";
import {
  type IStatusbarEntryItem,
  type IStatusbarService,
  StatusbarAlignment,
} from "../../../services/statusbar/browser/statusbar.js";

/** Browser view of the window-scoped status bar entry service. */
export class StatusbarPart extends WorkbenchPart {
  readonly #statusbarService: IStatusbarService;
  readonly #leftItems: HTMLDivElement;
  readonly #rightItems: HTMLDivElement;

  constructor(
    statusbarService: IStatusbarService,
    ownerDocument: Document,
  ) {
    super("statusbar", ownerDocument);
    this.#statusbarService = statusbarService;
    this.contentElement.setAttribute("role", "status");
    this.contentElement.setAttribute("aria-live", "polite");

    this.#leftItems = createItemsContainer(ownerDocument, "left");
    this.#rightItems = createItemsContainer(ownerDocument, "right");
    this.contentElement.append(this.#leftItems, this.#rightItems);

    this.own(this.#statusbarService.onDidChangeEntries(() => this.#render()));
    this.#render();
  }

  #render(): void {
    this.#renderItems(this.#leftItems, StatusbarAlignment.Left);
    this.#renderItems(this.#rightItems, StatusbarAlignment.Right);
  }

  #renderItems(
    container: HTMLDivElement,
    alignment: StatusbarAlignment,
  ): void {
    const elements = this.#statusbarService
      .getEntries(alignment)
      .map((item) => createEntryElement(container.ownerDocument, item));
    container.replaceChildren(...elements);
  }
}

function createItemsContainer(
  ownerDocument: Document,
  alignment: "left" | "right",
): HTMLDivElement {
  const container = ownerDocument.createElement("div");
  container.className =
    `zeta-statusbar-items zeta-statusbar-items-${alignment}`;
  return container;
}

function createEntryElement(
  ownerDocument: Document,
  item: IStatusbarEntryItem,
): HTMLSpanElement {
  const element = ownerDocument.createElement("span");
  element.className = "zeta-statusbar-item";
  element.dataset.statusbarItemId = item.id;
  element.textContent = item.entry.text;
  if (item.entry.ariaLabel) {
    element.setAttribute("aria-label", item.entry.ariaLabel);
  }
  if (item.entry.tooltip) {
    element.title = item.entry.tooltip;
  }
  return element;
}
