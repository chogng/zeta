import { type IDimension } from "../../base/browser/geometry.js";
import { Emitter, type Event } from "../../base/common/event.js";
import { DisposableOwner } from "../../base/common/lifecycle.js";

/**
 * Base class for a persistent visual region in the browser workbench shell.
 *
 * Parts own their layout constraints. WorkbenchLayout decides topology and
 * delegates the resulting pixel dimensions through `layout`.
 */
export abstract class WorkbenchPart extends DisposableOwner {
  readonly element: HTMLElement;
  protected readonly titleElement: HTMLDivElement;
  protected readonly contentElement: HTMLDivElement;
  readonly #onDidChangeConstraints = this.own(new Emitter<void>());

  readonly onDidChangeConstraints: Event<void> =
    this.#onDidChangeConstraints.event;

  protected constructor(id: string, ownerDocument: Document) {
    super();
    const element = ownerDocument.createElement("section");
    this.element = element;
    this.defer(() => element.remove());
    element.className = `zeta-workbench-part zeta-workbench-${id}`;
    element.dataset.part = id;
    this.titleElement = ownerDocument.createElement("div");
    this.titleElement.className = "zeta-workbench-part-title";
    this.contentElement = ownerDocument.createElement("div");
    this.contentElement.className = "zeta-workbench-part-content";
    element.append(this.titleElement, this.contentElement);
  }

  get minimumWidth(): number { return 0; }
  get maximumWidth(): number { return Number.POSITIVE_INFINITY; }
  get minimumHeight(): number { return 0; }
  get maximumHeight(): number { return Number.POSITIVE_INFINITY; }

  layout(_dimension: IDimension): void {}

  setVisible(visible: boolean): void {
    this.element.hidden = !visible;
  }

  /** Notifies the runtime layout after a subclass changes its constraints. */
  protected notifyConstraintsChanged(): void {
    this.#onDidChangeConstraints.fire();
  }
}
