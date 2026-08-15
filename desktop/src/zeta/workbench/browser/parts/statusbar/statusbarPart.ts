import "./statusbarpart.css";
import { WorkbenchPart } from "../../part.js";
import { StatusbarHeight } from "../workbenchPartDimensions.js";
import { StatusbarEntryItem } from "./statusbarItem.js";
import { type IStatusbarEntryItem, type IStatusbarService, StatusbarAlignment } from "../../../services/statusbar/browser/statusbar.js";

/** Browser view of the window-scoped status bar entry service. */
export class StatusbarPart extends WorkbenchPart {
  private readonly statusbarService: IStatusbarService;
  private readonly leftItems: HTMLDivElement;
  private readonly rightItems: HTMLDivElement;
  private readonly items = new Map<string, StatusbarEntryItem>();

  override get minimumHeight(): number { return StatusbarHeight; }
  override get maximumHeight(): number { return StatusbarHeight; }

  constructor(
    statusbarService: IStatusbarService,
    ownerDocument: Document,
  ) {
    super("statusbar", ownerDocument);
    this.statusbarService = statusbarService;
    this.contentElement.setAttribute("role", "status");
    this.contentElement.setAttribute("aria-live", "polite");
    this.contentElement.tabIndex = 0;

    this.leftItems = createItemsContainer(ownerDocument, "left");
    this.rightItems = createItemsContainer(ownerDocument, "right");
    this.contentElement.append(this.leftItems, this.rightItems);

    this.defer(() => this.disposeItems());
    this.own(this.statusbarService.onDidChangeEntries(() => this.render()));
    this.render();
  }

  private render(): void {
    const leftEntries = this.statusbarService.getEntries(StatusbarAlignment.Left);
    const rightEntries = this.statusbarService.getEntries(StatusbarAlignment.Right);
    const visibleIds = new Set([...leftEntries, ...rightEntries].map(({ id }) => id));
    for (const [id, item] of this.items) {
      if (visibleIds.has(id)) continue;
      item.dispose();
      this.items.delete(id);
    }
    this.renderItems(this.leftItems, leftEntries);
    this.renderItems(this.rightItems, rightEntries);
  }

  private renderItems(
    container: HTMLDivElement,
    entries: readonly IStatusbarEntryItem[],
  ): void {
    for (const entry of entries) {
      let item = this.items.get(entry.id);
      if (item) {
        item.update(entry.entry);
      } else {
        item = new StatusbarEntryItem(entry.id, entry.entry, container.ownerDocument);
        this.items.set(entry.id, item);
      }
      container.append(item.element);
    }
  }

  private disposeItems(): void {
    for (const item of this.items.values()) item.dispose();
    this.items.clear();
  }
}

function createItemsContainer(ownerDocument: Document, alignment: "left" | "right"): HTMLDivElement {
  const container = ownerDocument.createElement("div");
  container.className = `zeta-statusbar-items zeta-statusbar-items-${alignment}`;
  return container;
}
