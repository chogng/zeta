import "./media/gotoLineWidget.css";
import { registerEditorContribution } from "../../../browser/editorContribution.js";
import { addDisposableListener, stopEvent, h } from "../../../../base/browser/dom.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { operatingSystem, OperatingSystem } from "../../../../base/common/platform.js";
import { parseAsterGotoLocation, type GotoLocationParseResult } from "../../../common/commands/gotoLocation.js";
import { EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { TextSelection, TextSelectionSet } from "../../../common/core/selection.js";
import { type EditorScrollPosition } from "../../../common/viewLayout/editorViewportModel.js";
import { type EditorViewport } from "../../../browser/view/editorViewport.js";

export interface GotoLineControllerOptions {
  readonly operatingSystem?: OperatingSystem;
}

/** Owns Aster's local Go to Line/Column dialog and platform G shortcut. */
export class GotoLineController extends DisposableOwner {
  readonly element: HTMLDivElement;
  readonly input: HTMLInputElement;
  private readonly status: HTMLSpanElement;
  private initialScrollPosition: EditorScrollPosition | undefined;

  constructor(
    private readonly editorInput: HTMLTextAreaElement,
    private readonly viewport: EditorViewport,
    private readonly selections: EditorSelectionController,
    options: GotoLineControllerOptions = {},
  ) {
    super();
    this.targetOperatingSystem = options.operatingSystem ?? operatingSystem;
    if (viewport.textModel !== selections.textModel) {
      this.dispose();
      throw new TypeError("Aster Go to Line dependencies must share one text model");
    }
    const ownerDocument = viewport.element.ownerDocument;
    this.element = h(ownerDocument, "div");
    this.element.className = "aster-editor-goto-line-widget";
    this.element.hidden = true;
    this.element.setAttribute("role", "dialog");
    this.element.setAttribute("aria-label", "Go to Line or Column");
    this.input = h(ownerDocument, "input");
    this.input.className = "aster-editor-goto-line-input";
    this.input.type = "text";
    this.input.placeholder = "Line[:Column]";
    this.input.setAttribute("aria-label", "Line number and optional column");
    this.input.autocomplete = "off";
    this.input.spellcheck = false;
    this.status = h(ownerDocument, "span");
    this.status.className = "aster-editor-goto-line-status";
    this.status.setAttribute("aria-live", "polite");
    this.element.append(this.input, this.status);
    viewport.element.append(this.element);
    this.defer(() => this.element.remove());
    this.own(addDisposableListener(editorInput, "keydown", event => this.handleEditorKeydown(event)));
    this.own(addDisposableListener(this.element, "keydown", event => this.handleWidgetKeydown(event)));
    this.own(addDisposableListener(this.input, "input", () => this.preview()));
    this.own(viewport.onDidChangeLayout(() => this.position()));
    this.own(viewport.textModel.onDidChange(() => {
      if (this.visible) this.preview();
    }));
  }

  private readonly targetOperatingSystem: OperatingSystem;

  get visible(): boolean {
    return !this.element.hidden;
  }

  open(): void {
    if (!this.visible) this.initialScrollPosition = this.viewport.viewportLayout.scrollPosition;
    this.element.hidden = false;
    this.element.classList.add("visible");
    const current = this.selections.selections.primary.active;
    this.input.value = `${current.lineIndex + 1}:${current.columnIndex + 1}`;
    this.position();
    this.preview();
    this.input.focus({ preventScroll: true });
    this.input.select();
  }

  close(): void {
    if (!this.visible) return;
    this.element.hidden = true;
    this.element.classList.remove("visible");
    const initial = this.initialScrollPosition;
    this.initialScrollPosition = undefined;
    if (initial) this.viewport.scrollTo(initial);
    this.editorInput.focus({ preventScroll: true });
  }

  private handleEditorKeydown(event: KeyboardEvent): void {
    if (event.defaultPrevented || event.isComposing || event.getModifierState("AltGraph")) return;
    if (!isAsterGotoLineChord(event, this.targetOperatingSystem)) return;
    stopEvent(event);
    this.open();
  }

  private handleWidgetKeydown(event: KeyboardEvent): void {
    if (event.defaultPrevented || event.isComposing) return;
    if (event.key === "Escape") {
      stopEvent(event);
      this.close();
      return;
    }
    if (event.target !== this.input || event.key !== "Enter" || event.ctrlKey || event.metaKey || event.altKey) return;
    const result = this.readResult();
    if (result.kind !== "location") return;
    stopEvent(event);
    this.selections.setSelections(TextSelectionSet.single(TextSelection.collapsedAt(result.location.position)));
    this.viewport.revealPosition(result.location.position);
    this.initialScrollPosition = undefined;
    this.close();
  }

  private preview(): void {
    const result = this.readResult();
    this.status.textContent = result.message;
    this.input.classList.toggle("invalid", result.kind === "invalid");
    this.input.setAttribute("aria-invalid", String(result.kind === "invalid"));
    if (result.kind === "location") this.viewport.revealPosition(result.location.position);
  }

  private readResult(): GotoLocationParseResult {
    return parseAsterGotoLocation(this.viewport.textModel, this.input.value);
  }

  private position(): void {
    if (!this.visible) return;
    const layout = this.viewport.viewportLayout;
    const width = Math.max(0, Math.min(340, layout.viewportSize.width - 24));
    this.element.style.width = `${width}px`;
    this.element.style.left = `${layout.scrollPosition.left + Math.max(0, layout.viewportSize.width - width - 12)}px`;
    this.element.style.top = `${layout.scrollPosition.top + 6}px`;
  }
}

registerEditorContribution({
  id: "editor.contrib.quickAccess",
  install: context => {
    if (context.kind !== "text") return;
    context.own(new GotoLineController(context.textInput.element, context.viewport, context.selections));
  },
});

/** Identifies Ctrl+G on Windows/Linux and Command+G on macOS. */
export function isAsterGotoLineChord(event: Pick<KeyboardEvent, "key" | "ctrlKey" | "shiftKey" | "altKey" | "metaKey">, targetOperatingSystem: OperatingSystem): boolean {
  if (event.shiftKey || event.altKey || event.key.toLowerCase() !== "g") return false;
  return targetOperatingSystem === OperatingSystem.Macintosh
    ? event.metaKey && !event.ctrlKey
    : event.ctrlKey && !event.metaKey;
}
