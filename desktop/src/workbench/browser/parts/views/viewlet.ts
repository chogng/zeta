import { ViewPaneContainer } from "./viewPaneContainer.js";

/** A named primary view container that a SidebarPart can select and host. */
export class Viewlet extends ViewPaneContainer {
  readonly title: string;

  constructor(id: string, title: string) {
    super(id);
    this.title = title;
    this.element.classList.add("zeta-viewlet");
    this.element.setAttribute("aria-label", title);
  }
}
