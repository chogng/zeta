import type { IDimension } from "../../../base/browser/geometry.js";
import type { Event } from "../../../base/common/event.js";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
import type { EmbeddedTextEditorOptions, IEmbeddedTextEditor, IEmbeddedTextEditorFactory } from "../../../workbench/browser/parts/editor/embeddedTextEditor.js";

/**
 * A document `textBlock` projection backed by the line-oriented editor.
 *
 * The document model retains block identity and transactions. This widget only
 * hosts the embedded line editor and reports its text snapshot to the owner.
 */
export class TextEditorWidget extends DisposableOwner implements IEmbeddedTextEditor {
  private readonly editor: IEmbeddedTextEditor;
  readonly onDidChange: Event<string>;

  constructor(factory: IEmbeddedTextEditorFactory, options: EmbeddedTextEditorOptions) {
    super();
    if (!factory || typeof factory.create !== "function") throw new TypeError("Text editor widget requires an embedded editor factory");
    this.editor = this.own(factory.create(options));
    this.onDidChange = this.editor.onDidChange;
  }

  create(parent: HTMLElement): void {
    this.editor.create(parent);
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
