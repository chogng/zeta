import { WorkbenchPart } from "../../part.js";

/** The central content region that hosts the active workbench editor or view. */
export class EditorPart extends WorkbenchPart {
  constructor() {
    super("editor");
    this.element.setAttribute("aria-label", "Editor");
  }

  setContent(content: Element): void { this.contentElement.replaceChildren(content); }
}
