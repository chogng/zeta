import "./statusbarItem.css";
import { addDisposableListener, h, text as createText } from "../../../../base/browser/dom.js";
import { DisposableOwner, DisposableSlot } from "../../../../base/common/lifecycle.js";
import { getHoverDelegate, type IManagedHover } from "../../../../base/browser/ui/hover/hoverDelegate.js";
import { appendIcon } from "../../../../base/browser/ui/icon/icon.js";
import type { IStatusbarEntry, IStatusbarEntrySegment } from "../../../services/statusbar/browser/statusbar.js";

const StatusbarHoverGroupId = "statusbar";
type CompactHoverState = "none" | "group" | "entry";

/** Owns the DOM and interaction presentation for one status bar entry. */
export class StatusbarEntryItem extends DisposableOwner {
  readonly id: string;
  readonly element: HTMLDivElement;
  private readonly labelElement: HTMLAnchorElement;
  private readonly hover = this.own(new DisposableSlot<IManagedHover>());
  private iconElement: SVGElement | undefined;
  private textNode: Text | undefined;
  private segmentElements: HTMLElement[] = [];
  private entry: IStatusbarEntry | undefined;

  constructor(
    id: string,
    entry: IStatusbarEntry,
    ownerDocument: Document,
  ) {
    super();
    this.id = id;
    const element = h(ownerDocument, "div");
    element.className = "zeta-statusbar-item";
    element.dataset.statusbarItemId = id;
    this.element = element;

    const labelElement = h(ownerDocument, "a");
    labelElement.className = "zeta-statusbar-item-label";
    labelElement.setAttribute("role", "button");
    labelElement.tabIndex = -1;
    this.labelElement = labelElement;
    element.append(labelElement);

    this.own(addDisposableListener(labelElement, "click", (event) => {
      if (!this.isFocusable()) {
        event.preventDefault();
        return;
      }
      event.preventDefault();
      this.entry?.run?.();
    }));
    this.own(addDisposableListener(labelElement, "keydown", (event) => {
      if (event.key !== "Enter" && event.key !== " ") return;
      if (!this.isFocusable()) {
        event.preventDefault();
        return;
      }
      event.preventDefault();
      event.stopPropagation();
      this.entry?.run?.();
    }));

    this.update(entry);
  }

  /** Applies new content while retaining the item shell and its event handlers. */
  update(entry: IStatusbarEntry): void {
    const previousEntry = this.entry;
    this.entry = entry;
    const accessibleLabel = entry.ariaLabel || entry.text;
    const previousAccessibleLabel = previousEntry?.ariaLabel || previousEntry?.text;
    if (!previousEntry || previousAccessibleLabel !== accessibleLabel) {
      setOptionalAttribute(this.element, "aria-label", accessibleLabel);
      setOptionalAttribute(this.labelElement, "aria-label", accessibleLabel);
    }
    const focusable = this.isFocusable();
    const previouslyFocusable = previousEntry?.run !== undefined;
    if (!previousEntry || previouslyFocusable !== focusable) {
      this.labelElement.classList.toggle("disabled", !focusable);
      if (focusable) this.labelElement.removeAttribute("aria-disabled");
      else this.labelElement.setAttribute("aria-disabled", "true");
    }
    if (!previousEntry || previousEntry.kind !== entry.kind) {
      this.element.classList.toggle("remote-kind", entry.kind === "remote");
    }
    // The part is the single Tab stop. Items are focused by the part's
    // navigation commands, matching VS Code's composite statusbar behavior.
    this.labelElement.tabIndex = -1;

    this.updateContent(previousEntry, entry);

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

  isFocusable(): boolean {
    return this.entry?.run !== undefined;
  }

  isFocused(): boolean {
    const activeElement = this.element.ownerDocument.activeElement;
    return activeElement !== null && this.element.contains(activeElement);
  }

  focus(): void {
    if (this.isFocusable()) this.labelElement.focus();
  }

  hideHover(): void {
    this.hover.value?.hide();
  }

  setCompactNeighbors(neighbors: { readonly left: boolean; readonly right: boolean }): void {
    this.element.classList.toggle("compact-left", neighbors.left);
    this.element.classList.toggle("compact-right", neighbors.right);
  }

  setCompactHoverState(state: CompactHoverState): void {
    this.element.classList.toggle("compact-group-hover", state !== "none");
    this.element.classList.toggle("compact-entry-hover", state === "entry");
  }

  private updateContent(previousEntry: IStatusbarEntry | undefined, entry: IStatusbarEntry): void {
    this.element.classList.toggle("icon-only", entry.icon !== undefined && !entry.text && entry.segments === undefined);
    this.labelElement.classList.toggle("has-segments", entry.segments !== undefined);
    if (entry.segments) {
      this.iconElement?.remove();
      this.iconElement = undefined;
      this.textNode?.remove();
      this.textNode = undefined;
      this.updateSegments(previousEntry?.segments, entry.segments);
      return;
    }

    this.clearSegments();
    this.updateIcon(previousEntry?.segments ? undefined : previousEntry?.icon?.id, entry.icon);
    this.updateText(previousEntry?.segments ? undefined : previousEntry?.text, entry.text);
  }

  private updateSegments(previousSegments: readonly IStatusbarEntrySegment[] | undefined, segments: readonly IStatusbarEntrySegment[]): void {
    if (segmentsEqual(previousSegments, segments)) return;
    this.clearSegments();
    for (const segment of segments) {
      const segmentElement = h(this.labelElement.ownerDocument, "span");
      segmentElement.className = "zeta-statusbar-item-segment";
      if (segment.icon) appendIcon(segment.icon, segmentElement);
      if (segment.text) segmentElement.append(segment.text);
      this.labelElement.append(segmentElement);
      this.segmentElements.push(segmentElement);
    }
  }

  private clearSegments(): void {
    for (const element of this.segmentElements) element.remove();
    this.segmentElements = [];
  }

  private updateIcon(previousIconId: string | undefined, icon: IStatusbarEntry["icon"]): void {
    if (previousIconId === icon?.id) return;
    this.iconElement?.remove();
    this.iconElement = undefined;
    if (!icon) return;

    const iconElement = appendIcon(icon, this.labelElement);
    if (this.textNode) this.labelElement.insertBefore(iconElement, this.textNode);
    this.iconElement = iconElement;
  }

  private updateText(previousText: string | undefined, text: string): void {
    if (previousText === text) return;
    if (!text) {
      this.textNode?.remove();
      this.textNode = undefined;
      return;
    }
    if (this.textNode) {
      this.textNode.data = text;
      return;
    }
    this.textNode = createText(this.labelElement.ownerDocument, text);
    this.labelElement.append(this.textNode);
  }
}

function segmentsEqual(first: readonly IStatusbarEntrySegment[] | undefined, second: readonly IStatusbarEntrySegment[]): boolean {
  return first?.length === second.length && first.every((segment, index) => segment.icon?.id === second[index]?.icon?.id && segment.text === second[index]?.text);
}

function setOptionalAttribute(
  element: HTMLElement,
  name: string,
  value: string,
): void {
  if (value) element.setAttribute(name, value);
  else element.removeAttribute(name);
}
