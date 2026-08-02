import { addDisposableListener } from "../../dom.js";
import { trackFocus } from "../../focus.js";
import { appendIcon } from "../icon/icon.js";
import type { Event } from "../../../common/event.js";
import { DisposableOwner } from "../../../common/lifecycle.js";
import { lxiconsLibrary } from "../../../common/lxiconsLibrary.js";

/** Construction inputs for a titled, collapsible pane. */
export interface PaneViewOptions {
  readonly id: string;
  readonly title: string;
  readonly ownerDocument: Document;
  readonly collapsed?: boolean;
}

/**
 * Domain-agnostic titled pane that owns its header geometry, collapse state,
 * accessibility semantics, and title interaction.
 *
 * Consumers append domain content to {@link contentElement} and may add a
 * stable root class for their own outer presentation. They must not recreate
 * or style the header interaction internals.
 */
export class PaneView extends DisposableOwner {
  readonly element: HTMLElement;
  readonly id: string;
  protected readonly headerElement: HTMLDivElement;
  protected readonly contentElement: HTMLDivElement;
  private readonly headerButton: HTMLButtonElement;
  private readonly titleLabel: HTMLSpanElement;
  private readonly focusTracker;
  private collapsed: boolean;

  readonly onDidFocus: Event<void>;
  readonly onDidBlur: Event<void>;

  constructor(options: PaneViewOptions) {
    super();
    const { id, title, ownerDocument } = options;
    const element = ownerDocument.createElement("section");
    this.element = element;
    this.defer(() => element.remove());
    element.className = "zeta-pane-view";
    element.dataset.paneViewId = id;
    element.tabIndex = -1;
    this.id = id;

    this.headerElement = ownerDocument.createElement("div");
    this.headerElement.className = "zeta-pane-view-header";
    this.headerButton = ownerDocument.createElement("button");
    this.headerButton.className = "zeta-pane-view-header-button";
    this.headerButton.type = "button";
    const indicator = ownerDocument.createElement("span");
    indicator.className = "zeta-pane-view-header-chevron";
    indicator.setAttribute("aria-hidden", "true");
    const collapsedIcon = appendIcon(lxiconsLibrary.chevronRight, indicator);
    collapsedIcon.classList.add("zeta-pane-view-collapsed-icon");
    const expandedIcon = appendIcon(lxiconsLibrary.chevronDown, indicator);
    expandedIcon.classList.add("zeta-pane-view-expanded-icon");
    this.titleLabel = ownerDocument.createElement("span");
    this.titleLabel.className = "zeta-pane-view-header-label";
    this.titleLabel.textContent = title;
    this.headerButton.append(indicator, this.titleLabel);
    this.headerElement.append(this.headerButton);

    this.contentElement = ownerDocument.createElement("div");
    this.contentElement.className = "zeta-pane-view-content";
    this.contentElement.id = `zeta-pane-view-content-${encodeURIComponent(id)}`;
    this.headerButton.setAttribute("aria-controls", this.contentElement.id);
    element.append(this.headerElement, this.contentElement);
    this.collapsed = options.collapsed === true;
    this.renderCollapsedState();
    this.own(addDisposableListener(this.headerButton, "click", () => {
      this.setCollapsed(!this.collapsed);
    }));
    this.focusTracker = this.own(trackFocus(element));
    this.onDidFocus = this.focusTracker.onDidFocus;
    this.onDidBlur = this.focusTracker.onDidBlur;
  }

  setTitle(title: string): void {
    this.titleLabel.textContent = title;
  }

  isCollapsed(): boolean {
    return this.collapsed;
  }

  setCollapsed(collapsed: boolean): void {
    if (this.collapsed === collapsed) return;
    this.collapsed = collapsed;
    this.renderCollapsedState();
  }

  focus(): void {
    this.element.focus();
  }

  private renderCollapsedState(): void {
    const expanded = !this.collapsed;
    this.element.classList.toggle("collapsed", this.collapsed);
    this.headerButton.classList.toggle("expanded", expanded);
    this.headerButton.setAttribute("aria-expanded", String(expanded));
    this.contentElement.classList.toggle("collapsed", this.collapsed);
    this.contentElement.hidden = this.collapsed;
  }
}
