import "./media/debugBreakpointDecorations.css";
import { Emitter, type Event } from "../../../../base/common/event.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type URI } from "../../../../base/common/uri.js";
import { type EditorLineGutterDecoration } from "../../../../editor/browser/viewparts/margin/lineGutterDecoration.js";
import { type IDebugService } from "../../../services/debug/common/debugService.js";
import { h } from "../../../../base/browser/dom.js";

/** Workbench-owned projection of Debug breakpoints into the editor's generic gutter slot. */
export class DebugBreakpointDecorationProvider extends DisposableOwner implements EditorLineGutterDecoration {
  private readonly changeEmitter = this.own(new Emitter<void>());
  readonly onDidChange: Event<void> = this.changeEmitter.event;

  constructor(private readonly debug: IDebugService, private readonly resource: URI) {
    super();
    this.own(debug.onDidChangeBreakpoints(() => this.changeEmitter.fire()));
  }

  create(ownerDocument: Document): HTMLElement {
    const button = h(ownerDocument, "button");
    button.className = "zeta-debug-breakpoint-gutter";
    button.type = "button";
    button.addEventListener("click", () => {
      const lineIndex = Number(button.dataset.logicalLineIndex);
      if (Number.isSafeInteger(lineIndex) && lineIndex >= 0) this.debug.toggleBreakpoint(this.resource, lineIndex + 1);
    });
    return button;
  }

  project(element: HTMLElement, logicalLineIndex: number, firstForLogicalLine: boolean): void {
    if (!(element instanceof element.ownerDocument.defaultView!.HTMLButtonElement)) throw new TypeError("Debug breakpoint gutter requires a button element");
    const breakpoint = firstForLogicalLine ? this.debug.breakpoints.find(candidate => candidate.resource.toString() === this.resource.toString() && candidate.lineNumber === logicalLineIndex + 1) : undefined;
    element.hidden = !firstForLogicalLine;
    element.dataset.logicalLineIndex = String(logicalLineIndex);
    element.classList.toggle("checked", Boolean(breakpoint));
    element.classList.toggle("verified", breakpoint?.verified === true);
    element.setAttribute("aria-pressed", String(Boolean(breakpoint)));
    element.setAttribute("aria-label", breakpoint ? `Remove breakpoint at line ${logicalLineIndex + 1}` : `Add breakpoint at line ${logicalLineIndex + 1}`);
    element.title = element.getAttribute("aria-label") ?? "";
  }
}
