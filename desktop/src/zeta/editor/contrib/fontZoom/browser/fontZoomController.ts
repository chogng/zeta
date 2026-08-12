import "./media/fontZoom.css";
import { addDisposableListener, stopEvent } from "../../../../base/browser/dom.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type EditorViewport } from "../../../browser/view/editorViewport.js";

export interface FontZoomControllerOptions { readonly baseLineHeight?: number; readonly initialScale?: number; }

/** Owns per-editor font zoom state and invalidates browser measurements after each change. */
export class FontZoomController extends DisposableOwner {
  private readonly baseLineHeight: number;
  private scale: number;

  constructor(private readonly input: HTMLTextAreaElement, private readonly viewport: EditorViewport, options: FontZoomControllerOptions = {}) {
    super();
    this.baseLineHeight = readPositive(options.baseLineHeight ?? viewport.viewportLayout.lineHeight, "baseLineHeight");
    this.scale = readScale(options.initialScale ?? 1);
    this.apply();
    this.own(addDisposableListener(input, "keydown", event => this.handleKeydown(event), true));
  }

  get zoomScale(): number { return this.scale; }

  setZoomScale(scale: number): void {
    this.scale = readScale(scale);
    this.apply();
  }

  private handleKeydown(event: KeyboardEvent): void {
    if (event.defaultPrevented || event.isComposing || (!event.ctrlKey && !event.metaKey) || event.altKey || event.shiftKey) return;
    if (event.key === "+" || event.key === "=") { stopEvent(event, { immediate: true }); this.setZoomScale(this.scale + 0.1); }
    else if (event.key === "-") { stopEvent(event, { immediate: true }); this.setZoomScale(this.scale - 0.1); }
    else if (event.key === "0") { stopEvent(event, { immediate: true }); this.setZoomScale(1); }
  }

  private apply(): void {
    this.viewport.element.style.setProperty("--aster-editor-font-scale", String(this.scale));
    this.viewport.element.style.fontSize = `${this.scale}em`;
    this.viewport.setLineHeight(Math.max(1, Math.round(this.baseLineHeight * this.scale)));
    this.viewport.refreshFontMetrics();
    this.viewport.announceAccessibilityStatus(`Editor font size ${Math.round(this.scale * 100)} percent`);
  }
}

function readScale(value: number): number { if (!Number.isFinite(value) || value < 0.5 || value > 3) throw new RangeError("Aster font zoom scale must be between 0.5 and 3"); return Math.round(value * 10) / 10; }
function readPositive(value: number, name: string): number { if (!Number.isFinite(value) || value <= 0) throw new RangeError(`Aster ${name} must be positive`); return value; }
