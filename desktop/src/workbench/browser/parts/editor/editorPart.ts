import type { IElementView } from "../../../../base/browser/ui/index.js";
import { DisposableSlot } from "../../../../base/common/lifecycle.js";
import { WorkbenchPart } from "../../part.js";

/** The central content region that hosts the active workbench editor or view. */
export class EditorPart extends WorkbenchPart {
  readonly #view: DisposableSlot<IElementView>;

  constructor(ownerDocument: Document) {
    super("editor", ownerDocument);
    this.#view = this.own(new DisposableSlot<IElementView>());
    this.element.setAttribute("aria-label", "Editor");
  }

  setContent(content: Element): void {
    this.#view.clear();
    this.contentElement.replaceChildren(content);
  }

  setView(view: IElementView): void {
    this.#view.replace(view);
    this.contentElement.replaceChildren(view.element);
  }
}
