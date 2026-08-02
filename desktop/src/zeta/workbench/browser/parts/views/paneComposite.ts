import { compositePanelId, compositeTabId } from "../compositebar/compositeBar.js";
import { ViewPaneContainer, type ViewPaneContainerOptions } from "./viewPaneContainer.js";
import type { IAction } from "../../../../base/common/actions.js";

export interface PaneCompositeOptions extends ViewPaneContainerOptions {
  readonly paneHeaders?: PaneHeaderVisibility;
  readonly paneLayout?: PaneLayout;
}

export type PaneHeaderVisibility = "visible" | "hidden";
export type PaneLayout = "stack" | "fill";

/**
 * Activatable Composite whose content is assembled from registered ViewPanes.
 *
 * Parts retain instances while switching so pane visibility, focus, and
 * contribution-owned state survive temporary deactivation.
 */
export class PaneComposite extends ViewPaneContainer {
  readonly title: string;

  constructor(options: PaneCompositeOptions) {
    super(options);
    this.title = options.viewContainer.title;
    this.element.classList.add("zeta-pane-composite");
    this.element.classList.toggle("zeta-pane-composite-pane-headers-hidden", options.paneHeaders === "hidden");
    this.element.classList.toggle("zeta-pane-composite-pane-layout-fill", options.paneLayout === "fill");
    this.element.setAttribute("aria-label", this.title);
    this.element.id = compositePanelId(options.viewContainer.location, options.viewContainer.id);
    this.element.setAttribute("role", "tabpanel");
    this.element.setAttribute("aria-labelledby", compositeTabId(options.viewContainer.location, options.viewContainer.id));
  }

  get partTitleActionsElement(): HTMLElement | undefined {
    return this.panes.find((pane) => pane.partTitleActionsElement)?.partTitleActionsElement;
  }

  get partTitleElement(): HTMLElement | undefined {
    return this.panes.find((pane) => pane.partTitleElement)?.partTitleElement;
  }

  setTitleSecondaryActions(actions: readonly IAction[]): boolean {
    return this.panes.some((pane) => pane.setTitleSecondaryActions(actions));
  }
}
