import "./media/proseMirrorEditor.css";
import type {
  Node as ProseMirrorNode,
} from "prosemirror-model";
import {
  EditorView,
} from "prosemirror-view";
import type {
  IDimension,
} from "../../../base/browser/geometry.js";
import {
  DisposableOwner,
} from "../../../base/common/lifecycle.js";
import type {
  EditorInput,
} from "../../../workbench/browser/parts/editor/editorInput.js";
import type {
  IEditorPane,
} from "../../../workbench/browser/parts/editor/editorPane.js";
import {
  EditorPaneVisibility,
} from "../../../workbench/browser/parts/editor/editorPane.js";
import {
  PROSEMIRROR_EDITOR_ID,
} from "../common/proseMirrorEditorInput.js";
import {
  createProseMirrorEditorState,
} from "../common/proseMirrorEditorState.js";

/** Browser host for the customizable ProseMirror editor subsystem. */
export class ProseMirrorEditorPane extends DisposableOwner
  implements IEditorPane {
  readonly id = PROSEMIRROR_EDITOR_ID;

  private container: HTMLDivElement | undefined;
  private view: EditorView | undefined;

  create(parent: HTMLElement): void {
    if (this.container) {
      throw new ReferenceError(
        "ProseMirrorEditorPane has already been created",
      );
    }
    const container = parent.ownerDocument.createElement("div");
    container.className = "zeta-prosemirror-editor-pane";
    parent.append(container);
    this.container = container;
    this.defer(() => {
      this.destroyView();
      container.remove();
      this.container = undefined;
    });
  }

  async setInput(
    input: EditorInput,
    signal: AbortSignal,
  ): Promise<void> {
    const container = this.requireContainer();
    throwIfAborted(signal);
    const state = createProseMirrorEditorState(input.initialText ?? "");
    throwIfAborted(signal);
    this.destroyView();
    const view = new EditorView(container, {
      state,
      attributes: {
        "aria-label": input.label ?? resourceLabel(input),
        class: "zeta-prosemirror-editor-surface",
        spellcheck: "true",
      },
    });
    if (signal.aborted) {
      view.destroy();
      throw abortError();
    }
    this.view = view;
  }

  clearInput(): void {
    this.destroyView();
  }

  layout(dimension: IDimension): void {
    const container = this.container;
    if (!container) return;
    container.style.width = `${Math.max(0, dimension.width)}px`;
    container.style.height = `${Math.max(0, dimension.height)}px`;
  }

  setVisible(visibility: EditorPaneVisibility): void {
    if (this.container) {
      this.container.hidden =
        visibility === EditorPaneVisibility.Hidden;
    }
  }

  focus(): void {
    const view = this.view;
    if (!view) {
      throw new ReferenceError(
        "ProseMirrorEditorPane has no active input",
      );
    }
    view.focus();
  }

  getDocument(): ProseMirrorNode {
    const view = this.view;
    if (!view) {
      throw new ReferenceError(
        "ProseMirrorEditorPane has no active input",
      );
    }
    return view.state.doc;
  }

  private destroyView(): void {
    this.view?.destroy();
    this.view = undefined;
    this.container?.replaceChildren();
  }

  private requireContainer(): HTMLDivElement {
    if (!this.container) {
      throw new ReferenceError(
        "ProseMirrorEditorPane has not been created",
      );
    }
    return this.container;
  }
}

function resourceLabel(input: EditorInput): string {
  const path = decodeURIComponent(input.resource.path);
  return path.slice(path.lastIndexOf("/") + 1) || "Academic editor";
}

function throwIfAborted(signal: AbortSignal): void {
  if (signal.aborted) throw abortError();
}

function abortError(): DOMException {
  return new DOMException("Editor input loading was aborted", "AbortError");
}
