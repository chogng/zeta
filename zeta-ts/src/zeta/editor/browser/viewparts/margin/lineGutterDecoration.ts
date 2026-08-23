import { Emitter, type Event } from "../../../../base/common/event.js";
import { DisposableOwner, type IDisposable } from "../../../../base/common/lifecycle.js";
import { h } from "../../../../base/browser/dom.js";

export const EDITOR_GUTTER_SLOT_WIDTH = 20;

/** Optional feature-owned projection hosted in one rendered line's margin slot. */
export interface EditorLineGutterDecoration extends IDisposable {
  readonly onDidChange: Event<void>;
  create(ownerDocument: Document): HTMLElement;
  project(element: HTMLElement, logicalLineIndex: number, firstForLogicalLine: boolean): void;
}

/** Owns and projects independently contributed margin decorations without making features aware of each other. */
export class CompositeEditorLineGutterDecoration extends DisposableOwner implements EditorLineGutterDecoration {
  private readonly changeEmitter = this.own(new Emitter<void>());
  readonly onDidChange: Event<void> = this.changeEmitter.event;

  constructor(readonly decorations: readonly EditorLineGutterDecoration[]) {
    super();
    if (decorations.length === 0) throw new RangeError("A gutter decoration composite requires at least one decoration");
    for (const decoration of decorations) {
      this.own(decoration);
      this.own(decoration.onDidChange(() => this.changeEmitter.fire()));
    }
  }

  get width(): number { return this.decorations.length * EDITOR_GUTTER_SLOT_WIDTH; }

  create(ownerDocument: Document): HTMLElement {
    const root = h(ownerDocument, "span");
    root.className = "aster-editor-feature-gutter";
    for (const decoration of this.decorations) {
      const slot = h(ownerDocument, "span");
      slot.className = "aster-editor-feature-gutter-slot";
      slot.append(decoration.create(ownerDocument));
      root.append(slot);
    }
    return root;
  }

  project(element: HTMLElement, logicalLineIndex: number, firstForLogicalLine: boolean): void {
    if (element.children.length !== this.decorations.length) throw new Error("Editor gutter decoration DOM is out of sync");
    this.decorations.forEach((decoration, index) => {
      const slot = element.children[index];
      const target = slot?.firstElementChild;
      if (!(target instanceof element.ownerDocument.defaultView!.HTMLElement)) throw new TypeError("Editor gutter decoration slot is invalid");
      decoration.project(target, logicalLineIndex, firstForLogicalLine);
    });
  }
}

export function combineEditorLineGutterDecorations(decorations: readonly EditorLineGutterDecoration[]): CompositeEditorLineGutterDecoration | undefined {
  return decorations.length === 0 ? undefined : new CompositeEditorLineGutterDecoration(Object.freeze([...decorations]));
}
