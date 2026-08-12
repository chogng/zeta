import "./media/colorPicker.css";
import { addDisposableListener, stopEvent } from "../../../../base/browser/dom.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { RGBA8 } from "../../../common/core/misc/rgba.js";
import { createEditorEditCommand } from "../../../common/commands/editorCommand.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { type EditorViewport } from "../../../browser/view/editorViewport.js";
import { type ColorService, type LanguageColorInformation } from "../common/color.js";

/** Presents provider colors through a native color input and applies the selected text presentation. */
export class ColorPickerController extends DisposableOwner {
  private readonly element: HTMLDivElement;
  private readonly input: HTMLInputElement;
  private request: AbortController | undefined;
  private colors: readonly LanguageColorInformation[] = [];

  constructor(private readonly editorInput: HTMLTextAreaElement, private readonly viewport: EditorViewport, private readonly selections: EditorSelectionController, private readonly service: ColorService, private readonly languageId: string, private readonly onError: (error: unknown) => void = error => console.error("Aster color picker failed", error)) {
    super();
    if (viewport.textModel !== selections.textModel) throw new TypeError("Aster color picker dependencies must share a text model");
    const document = viewport.element.ownerDocument;
    this.element = document.createElement("div");
    this.element.className = "aster-editor-color-picker";
    this.element.hidden = true;
    this.input = document.createElement("input");
    this.input.type = "color";
    this.input.setAttribute("aria-label", "Choose color");
    this.element.append(this.input);
    viewport.element.append(this.element);
    this.defer(() => this.element.remove());
    this.own(addDisposableListener(editorInput, "keydown", event => { if (event.defaultPrevented || event.isComposing || !event.shiftKey || (!event.ctrlKey && !event.metaKey) || event.altKey || event.key.toLowerCase() !== "c") return; stopEvent(event); void this.open(); }, true));
    this.own(addDisposableListener(this.input, "change", () => void this.apply()));
    this.own(addDisposableListener(this.input, "keydown", event => { if (event.key === "Escape") { stopEvent(event); this.close(); } }));
    this.own(viewport.textModel.onDidChange(() => this.close()));
  }

  private async open(): Promise<void> {
    this.request?.abort();
    const request = this.request = new AbortController();
    try {
      this.colors = await this.service.provideDocumentColors(this.languageId, request.signal);
      if (request.signal.aborted) return;
      const active = this.selections.selections.primary.active;
      const color = this.colors.find(candidate => candidate.range.containsPosition(active));
      if (!color) { this.viewport.announceAccessibilityStatus("No color is available at the cursor."); return; }
      this.input.value = rgbToHex(color.color);
      const coordinates = this.viewport.getPositionContentCoordinates(color.range.start);
      const scroll = this.viewport.viewportLayout.scrollPosition;
      this.element.style.left = `${Math.max(4, coordinates.left - scroll.left)}px`;
      this.element.style.top = `${Math.max(4, coordinates.top - scroll.top + coordinates.height + 2)}px`;
      this.element.hidden = false;
      this.input.focus({ preventScroll: true });
    } catch (error) {
      if (!request.signal.aborted) this.onError(error);
    }
  }

  private async apply(): Promise<void> {
    const request = this.request;
    try {
      const active = this.selections.selections.primary.active;
      const color = this.colors.find(candidate => candidate.range.containsPosition(active));
      if (!color) return;
      const next = hexToRgb(this.input.value);
      const presentations = await this.service.provideColorPresentations(this.languageId, color.range, next, request?.signal);
      if (request?.signal.aborted) return;
      const presentation = presentations[0];
      if (presentation?.textEdit) {
        const edits = [presentation.textEdit, ...(presentation.additionalTextEdits ?? [])].sort((left, right) => left.range.start.compareTo(right.range.start));
        const command = createEditorEditCommand(this.viewport.textModel, this.selections.selections, edits);
        if (command) this.selections.execute(command);
      }
      this.close();
    } catch (error) {
      if (!request?.signal.aborted) this.onError(error);
    }
  }

  private close(): void { this.request?.abort(); this.request = undefined; this.element.hidden = true; this.editorInput.focus({ preventScroll: true }); }
}

function rgbToHex(color: RGBA8): string { return `#${[color.r, color.g, color.b].map(value => value.toString(16).padStart(2, "0")).join("")}`; }
function hexToRgb(value: string): RGBA8 { const normalized = value.replace(/^#/, ""); if (!/^[0-9a-f]{6}$/iu.test(normalized)) throw new TypeError("Color picker returned an invalid color"); return new RGBA8(Number.parseInt(normalized.slice(0, 2), 16), Number.parseInt(normalized.slice(2, 4), 16), Number.parseInt(normalized.slice(4, 6), 16), 255); }
