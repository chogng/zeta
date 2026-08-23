import type { IDimension } from "../../../base/browser/geometry.js";
import type { Event } from "../../../base/common/event.js";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
import type { EmbeddedTextEditorOptions, IEmbeddedTextEditor, IEmbeddedTextEditorFactory } from "./embeddedTextEditor.js";
import { h } from "../../../base/browser/dom.js";

/**
 * A document `textBlock` projection backed by the line-oriented editor.
 *
 * The document model retains block identity and transactions. This widget only
 * hosts the embedded line editor and reports its text snapshot to the owner.
 */
export class TextEditorWidget extends DisposableOwner implements IEmbeddedTextEditor {
  private readonly editor: IEmbeddedTextEditor;
  private container: HTMLDivElement | undefined;
  readonly onDidChange: Event<string>;

  constructor(factory: IEmbeddedTextEditorFactory, options: EmbeddedTextEditorOptions) {
    super();
    if (!factory || typeof factory.create !== "function") throw new TypeError("Text editor widget requires an embedded editor factory");
    this.editor = this.own(factory.create(options));
    this.onDidChange = this.editor.onDidChange;
  }

  create(parent: HTMLElement): void {
    if (this.container) throw new ReferenceError("Text editor widget has already been created");
    const container = h(parent.ownerDocument, "div");
    container.className = "zeta-document-embedded-text-editor";
    parent.append(container);
    this.container = container;
    this.editor.create(container);
    this.defer(() => {
      container.remove();
      this.container = undefined;
    });
  }

  setValue(value: string): void {
    this.editor.setValue(value);
  }

  getValue(): string {
    return this.editor.getValue();
  }

  layout(dimension: IDimension): void {
    this.editor.layout(dimension);
  }

  focus(): void {
    this.editor.focus();
  }
}
