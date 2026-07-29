import { WorkbenchPart } from "../part.js";
import { PaneComposite } from "./views/paneComposite.js";

/**
 * Workbench Part that retains and activates one PaneComposite at a time.
 *
 * Subclasses provide the surrounding region layout and may place a
 * CompositeBar independently. The shared title and content areas always
 * describe and host the active Composite.
 */
export abstract class CompositePart extends WorkbenchPart {
  readonly #composites = new Map<string, PaneComposite>();
  readonly #titleLabel: HTMLHeadingElement;
  #activeComposite: PaneComposite | undefined;

  protected constructor(id: string, ownerDocument: Document) {
    super(id, ownerDocument);
    this.titleElement.classList.add("zeta-composite-title");
    this.contentElement.classList.add("zeta-composite-content");
    this.#titleLabel = ownerDocument.createElement("h2");
    this.#titleLabel.className = "zeta-composite-title-label";
    this.titleElement.append(this.#titleLabel);
    this.defer(() => this.#composites.clear());
  }

  addComposite(composite: PaneComposite): void {
    if (this.#composites.has(composite.id)) {
      throw new Error(`Composite already exists in Part: ${composite.id}`);
    }
    this.#composites.set(composite.id, this.own(composite));
    composite.setVisible(false);
    this.contentElement.append(composite.element);
  }

  getComposite(compositeId: string): PaneComposite | undefined {
    return this.#composites.get(compositeId);
  }

  showComposite(compositeId: string): void {
    const composite = this.#composites.get(compositeId);
    if (!composite) {
      throw new Error(`Composite is not available in Part: ${compositeId}`);
    }
    if (this.#activeComposite === composite) return;
    this.#activeComposite?.setVisible(false);
    this.#activeComposite = composite;
    this.#titleLabel.textContent = composite.title;
    composite.setVisible(true);
  }

  get activeCompositeId(): string | undefined {
    return this.#activeComposite?.id;
  }
}
