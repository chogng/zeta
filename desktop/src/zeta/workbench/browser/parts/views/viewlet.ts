import {
  ViewPaneContainer,
  type ViewPaneContainerOptions,
} from "./viewPaneContainer.js";

/** A named primary view container that a SidebarPart can select and host. */
export class Viewlet extends ViewPaneContainer {
  readonly title: string;

  constructor(options: ViewPaneContainerOptions) {
    super(options);
    this.title = options.viewContainer.title;
    this.element.classList.add("zeta-viewlet");
    this.element.setAttribute("aria-label", this.title);
  }
}
