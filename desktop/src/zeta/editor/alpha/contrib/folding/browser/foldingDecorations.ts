import "./media/folding.css";
import { Emitter, type Event } from "../../../../../base/common/event.js";
import { DisposableOwner } from "../../../../../base/common/lifecycle.js";
import { type EditorFoldingModel } from "./foldingModel.js";
import { type EditorFoldingRegion } from "./foldingRanges.js";

/** Owns folding gutter presentation and mirrors every fold-state change. */
export class AlphaFoldingDecorationProvider extends DisposableOwner {
  private readonly changeEmitter = this.own(new Emitter<void>());

  readonly onDidChange: Event<void> = this.changeEmitter.event;

  constructor(private readonly folding: EditorFoldingModel | undefined) {
    super();
    if (folding) this.own(folding.onDidChange(() => this.changeEmitter.fire()));
  }

  project(element: HTMLButtonElement, logicalLineIndex: number, firstForLogicalLine: boolean): void {
    projectAlphaFoldingDecoration(
      element,
      logicalLineIndex,
      firstForLogicalLine
        ? this.folding?.regions.find(region => region.startLineIndex === logicalLineIndex)
        : undefined,
    );
  }
}

/** Creates the folding gutter control attached to an Alpha rendered line. */
export function createAlphaFoldingDecoration(ownerDocument: Document): HTMLButtonElement {
  const element = ownerDocument.createElement("button");
  element.className = "zeta-alpha-editor-fold-toggle";
  element.type = "button";
  element.hidden = true;
  return element;
}

/** Projects one folding region's semantic state onto its gutter control. */
export function projectAlphaFoldingDecoration(element: HTMLButtonElement, logicalLineIndex: number, region: EditorFoldingRegion | undefined): void {
  element.hidden = !region;
  if (!region) {
    delete element.dataset.logicalLineIndex;
    element.classList.remove("collapsed");
    element.removeAttribute("aria-expanded");
    element.removeAttribute("aria-label");
    element.textContent = "";
    return;
  }
  element.dataset.logicalLineIndex = String(logicalLineIndex);
  element.classList.toggle("collapsed", region.collapsed);
  element.setAttribute("aria-expanded", String(!region.collapsed));
  element.setAttribute("aria-label", region.collapsed ? "Expand folded lines" : "Collapse lines");
  element.textContent = region.collapsed ? "›" : "⌄";
}
