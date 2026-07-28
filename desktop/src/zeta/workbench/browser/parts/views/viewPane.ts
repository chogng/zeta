import "./views.css";
import { trackFocus } from "../../../../base/browser/focus.js";
import type { Event } from "../../../../base/common/event.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import type { IView } from "../../../common/views.js";

/** Runtime inputs supplied by a browser view container to every pane. */
export interface IViewPaneOptions {
  readonly id: string;
  readonly title: string;
  readonly ownerDocument: Document;
}

/** A titled, independently managed view hosted inside a workbench view container. */
export abstract class ViewPane extends DisposableOwner implements IView {
  readonly element: HTMLElement;
  readonly id: string;
  protected readonly titleElement: HTMLDivElement;
  protected readonly contentElement: HTMLDivElement;
  readonly #focusTracker;
  #visible = true;

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
    this.titleElement.textContent = title;
    this.contentElement = ownerDocument.createElement("div");
    this.contentElement.className = "zeta-view-pane-content";
    element.append(this.titleElement, this.contentElement);
    this.#focusTracker = this.own(trackFocus(element));
    this.onDidFocus = this.#focusTracker.onDidFocus;
    this.onDidBlur = this.#focusTracker.onDidBlur;
  }

  setTitle(title: string): void { this.titleElement.textContent = title; }

  focus(): void {
    this.element.focus();
  }

  isVisible(): boolean {
    return this.#visible;
  }

  setVisible(visible: boolean): void {
    if (this.#visible === visible) return;
    this.#visible = visible;
    this.element.hidden = !visible;
  }
}
