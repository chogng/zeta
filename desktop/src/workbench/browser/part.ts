import { Component } from "../../base/browser/ui/index.js";

/** Base class for a persistent visual region in the browser workbench shell. */
export abstract class WorkbenchPart extends Component<HTMLElement> {
  protected readonly titleElement: HTMLDivElement;
  protected readonly contentElement: HTMLDivElement;

  protected constructor(id: string) {
    const element = document.createElement("section");
    element.className = `zeta-workbench-part zeta-workbench-${id}`;
    element.dataset.part = id;
    super(element);
    this.titleElement = document.createElement("div");
    this.titleElement.className = "zeta-workbench-part-title";
    this.contentElement = document.createElement("div");
    this.contentElement.className = "zeta-workbench-part-content";
    element.append(this.titleElement, this.contentElement);
  }
}
