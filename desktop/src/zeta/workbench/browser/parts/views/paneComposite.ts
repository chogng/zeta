import {
  ViewPaneContainer,
  type ViewPaneContainerOptions,
} from "./viewPaneContainer.js";

/**
 * Activatable Composite whose content is assembled from registered ViewPanes.
 *
 * Parts retain instances while switching so pane visibility, focus, and
 * contribution-owned state survive temporary deactivation.
 */
export class PaneComposite extends ViewPaneContainer {
  readonly title: string;

  constructor(options: ViewPaneContainerOptions) {
    super(options);
    this.title = options.viewContainer.title;
    this.element.classList.add("zeta-pane-composite");
    this.element.setAttribute("aria-label", this.title);
  }
}
