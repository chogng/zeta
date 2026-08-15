import "./statusbarItem.css";
import { addDisposableListener } from "../../../../base/browser/dom.js";
import { DisposableOwner, DisposableSlot } from "../../../../base/common/lifecycle.js";
import { getHoverDelegate, type IManagedHover } from "../../../../base/browser/ui/hover/hoverDelegate.js";
import { appendIcon } from "../../../../base/browser/ui/icon/icon.js";
import type { IStatusbarEntry } from "../../../services/statusbar/browser/statusbar.js";

const StatusbarHoverGroupId = "statusbar";

/** Owns the DOM and interaction presentation for one status bar entry. */
export class StatusbarEntryItem extends DisposableOwner {
  readonly element: HTMLDivElement;
  private readonly labelElement: HTMLAnchorElement;
  private readonly hover = this.own(new DisposableSlot<IManagedHover>());
  private entry: IStatusbarEntry | undefined;

  constructor(
    id: string,
    entry: IStatusbarEntry,
    ownerDocument: Document,
  ) {
    super();
    const element = ownerDocument.createElement("div");
    element.className = "zeta-statusbar-item";
    element.dataset.statusbarItemId = id;
    this.element = element;

    const labelElement = ownerDocument.createElement("a");
    labelElement.className = "zeta-statusbar-item-label";
    labelElement.setAttribute("role", "button");
    labelElement.tabIndex = -1;
    this.labelElement = labelElement;
    element.append(labelElement);

    this.own(addDisposableListener(labelElement, "click", (event) => {
      if (!this.entry?.run) {
        event.preventDefault();
        return;
      }
      event.preventDefault();
      this.entry.run();
    }));
    this.own(addDisposableListener(labelElement, "keydown", (event) => {
      if (event.key !== "Enter" && event.key !== " ") return;
      if (!this.entry?.run) {
        event.preventDefault();
        return;
      }
      event.preventDefault();
      event.stopPropagation();
      this.entry.run();
    }));

    this.update(entry);
  }

  /** Applies new content while retaining the item shell and its event handlers. */
  update(entry: IStatusbarEntry): void {
    const previousEntry = this.entry;
    this.entry = entry;
    const accessibleLabel = entry.ariaLabel || entry.text;
    setOptionalAttribute(this.element, "aria-label", accessibleLabel);
    setOptionalAttribute(this.labelElement, "aria-label", accessibleLabel);
    this.labelElement.classList.toggle("disabled", entry.run === undefined);
    this.labelElement.toggleAttribute("aria-disabled", entry.run === undefined);
    this.labelElement.tabIndex = entry.run === undefined ? -1 : 0;

    this.labelElement.replaceChildren();
    if (entry.icon) appendIcon(entry.icon, this.labelElement);
    if (entry.text) this.labelElement.append(this.labelElement.ownerDocument.createTextNode(entry.text));

    if (!previousEntry || previousEntry.tooltip !== entry.tooltip) {
      this.hover.replace(entry.tooltip
        ? getHoverDelegate().setupHover({
          target: this.labelElement,
          content: entry.tooltip,
          groupId: StatusbarHoverGroupId,
        })
        : undefined);
    }
  }
}

function setOptionalAttribute(
  element: HTMLElement,
  name: string,
  value: string,
): void {
  if (value) element.setAttribute(name, value);
  else element.removeAttribute(name);
}
