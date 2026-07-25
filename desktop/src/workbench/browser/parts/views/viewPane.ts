import { Component } from "../../../../base/browser/ui/index.js";

/** A titled, independently managed view hosted inside a workbench view container. */
export abstract class ViewPane extends Component<HTMLElement> {
  readonly id: string;
  protected readonly titleElement: HTMLDivElement;
  protected readonly contentElement: HTMLDivElement;

  protected constructor(id: string, title: string) {
    const element = document.createElement("section");
    element.className = "zeta-view-pane";
    element.dataset.viewId = id;
    super(element);
    this.id = id;
    this.titleElement = document.createElement("div");
    this.titleElement.className = "zeta-view-pane-title";
    this.titleElement.textContent = title;
    this.contentElement = document.createElement("div");
    this.contentElement.className = "zeta-view-pane-content";
    element.append(this.titleElement, this.contentElement);
  }

  setTitle(title: string): void { this.titleElement.textContent = title; }
}
