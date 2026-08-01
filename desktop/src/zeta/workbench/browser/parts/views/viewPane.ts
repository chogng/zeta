import "./views.css";
import { addDisposableListener } from "../../../../base/browser/dom.js";
import { appendIcon } from "../../../../base/browser/ui/icon/icon.js";
import { trackFocus } from "../../../../base/browser/focus.js";
import type { Event } from "../../../../base/common/event.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { lxiconsLibrary } from "../../../../base/common/lxiconsLibrary.js";
import type { IView } from "../../../common/views.js";
import type { IAction } from "../../../../base/common/actions.js";

/** Runtime inputs supplied by a browser view container to every pane. */
export interface IViewPaneOptions {
  readonly id: string;
  readonly title: string;
  readonly ownerDocument: Document;
  readonly collapsed?: boolean;
}

/** A titled, independently managed view hosted inside a workbench view container. */
export abstract class ViewPane extends DisposableOwner implements IView {
  readonly element: HTMLElement;
  readonly id: string;
  protected readonly titleElement: HTMLDivElement;
  protected readonly contentElement: HTMLDivElement;
  private readonly titleButton: HTMLButtonElement;
  private readonly titleLabel: HTMLSpanElement;
  private readonly focusTracker;
  private visible = true;
  private collapsed: boolean;

  readonly onDidFocus: Event<void>;
  readonly onDidBlur: Event<void>;

  protected constructor(options: IViewPaneOptions) {
    super();
    const { id, title, ownerDocument } = options;
    const element = ownerDocument.createElement("section");
    this.element = element;
    this.defer(() => element.remove());
    element.className = "zeta-view-pane";
    element.dataset.viewId = id;
    element.tabIndex = -1;
    this.id = id;
    this.titleElement = ownerDocument.createElement("div");
    this.titleElement.className = "zeta-view-pane-title";
    this.titleButton = ownerDocument.createElement("button");
    this.titleButton.className = "zeta-view-pane-title-button";
    this.titleButton.type = "button";
    const indicator = ownerDocument.createElement("span");
    indicator.className = "zeta-view-pane-title-chevron";
    indicator.setAttribute("aria-hidden", "true");
    const collapsedIcon = appendIcon(lxiconsLibrary.chevronRight, indicator);
    collapsedIcon.classList.add("zeta-view-pane-collapsed-icon");
    const expandedIcon = appendIcon(lxiconsLibrary.chevronDown, indicator);
    expandedIcon.classList.add("zeta-view-pane-expanded-icon");
    this.titleLabel = ownerDocument.createElement("span");
    this.titleLabel.className = "zeta-view-pane-title-label";
    this.titleLabel.textContent = title;
    this.titleButton.append(indicator, this.titleLabel);
    this.titleElement.append(this.titleButton);
    this.contentElement = ownerDocument.createElement("div");
    this.contentElement.className = "zeta-view-pane-content";
    this.contentElement.id = `zeta-view-pane-content-${encodeURIComponent(id)}`;
    this.titleButton.setAttribute("aria-controls", this.contentElement.id);
    element.append(this.titleElement, this.contentElement);
    this.collapsed = options.collapsed === true;
    this.renderCollapsedState();
    this.own(addDisposableListener(this.titleButton, "click", () => {
      this.setCollapsed(!this.collapsed);
    }));
    this.focusTracker = this.own(trackFocus(element));
    this.onDidFocus = this.focusTracker.onDidFocus;
    this.onDidBlur = this.focusTracker.onDidBlur;
  }

  setTitle(title: string): void { this.titleLabel.textContent = title; }

  isCollapsed(): boolean {
    return this.collapsed;
  }

  setCollapsed(collapsed: boolean): void {
    if (this.collapsed === collapsed) return;
    this.collapsed = collapsed;
    this.renderCollapsedState();
  }

  /** Contextual commands rendered by a host title toolbar, when available. */
  get titleActionsElement(): HTMLElement | undefined {
    return undefined;
  }

  /** Lets a hosting Part merge its overflow commands into this title toolbar. */
  setTitleSecondaryActions(_actions: readonly IAction[]): boolean {
    return false;
  }

  focus(): void {
    this.element.focus();
  }

  isVisible(): boolean {
    return this.visible;
  }

  setVisible(visible: boolean): void {
    if (this.visible === visible) return;
    this.visible = visible;
    this.element.hidden = !visible;
  }

  private renderCollapsedState(): void {
    const expanded = !this.collapsed;
    this.element.classList.toggle("collapsed", this.collapsed);
    this.titleButton.classList.toggle("expanded", expanded);
    this.titleButton.setAttribute("aria-expanded", String(expanded));
    this.contentElement.hidden = this.collapsed;
  }
}
