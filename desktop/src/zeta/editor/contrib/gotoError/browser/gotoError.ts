import { addDisposableListener, stopEvent } from "../../../../base/browser/dom.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { TextDecorationCollection } from "../../../common/model/decorationCollection.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { type LanguageDiagnostic } from "../../../common/languages/languageResults.js";
import { TextSelection, TextSelectionSet } from "../../../common/core/selection.js";
import { type TextPosition } from "../../../common/core/text.js";
import { type EditorViewport } from "../../../browser/view/editorViewport.js";

/** Moves the primary selection through current-version diagnostics with F8. */
export class DiagnosticNavigationController extends DisposableOwner {
  constructor(input: HTMLTextAreaElement, private readonly viewport: EditorViewport, private readonly selections: EditorSelectionController, private readonly diagnostics: TextDecorationCollection<LanguageDiagnostic>) {
    super();
    if (viewport.textModel !== selections.textModel || diagnostics.textModel !== selections.textModel) {
      this.dispose();
      throw new TypeError("Alpha diagnostic navigation dependencies must share one text model");
    }
    this.own(addDisposableListener(input, "keydown", event => this.handleKeydown(event)));
  }

  private handleKeydown(event: KeyboardEvent): void {
    if (event.defaultPrevented || event.isComposing || event.ctrlKey || event.metaKey || event.altKey || event.key !== "F8") return;
    const diagnostics = this.diagnostics.decorations;
    if (diagnostics.length === 0) return;
    stopEvent(event);
    const active = this.selections.selections.primary.active;
    const direction = event.shiftKey ? -1 : 1;
    const index = direction > 0
      ? diagnostics.findIndex(diagnostic => diagnostic.range.start.compareTo(active) > 0)
      : findPreviousDiagnostic(diagnostics, active);
    const target = diagnostics[index === -1 ? (direction > 0 ? 0 : diagnostics.length - 1) : index]!;
    this.selections.setSelections(TextSelectionSet.single(TextSelection.from(target.range.start, target.range.end)));
    this.viewport.revealPosition(target.range.start);
    this.viewport.announceAccessibilityStatus(describeDiagnostic(target.metadata));
  }
}

function describeDiagnostic(diagnostic: LanguageDiagnostic): string {
  const source = [diagnostic.source, diagnostic.code].filter(value => value !== undefined).join(" ");
  const prefix = source.length === 0 ? diagnostic.severity : `${diagnostic.severity} ${source}`;
  return `${prefix}: ${diagnostic.message}`;
}

function findPreviousDiagnostic(diagnostics: readonly { readonly range: { readonly end: TextPosition } }[], active: TextPosition): number {
  for (let index = diagnostics.length - 1; index >= 0; index -= 1) {
    if (diagnostics[index]!.range.end.compareTo(active) < 0) return index;
  }
  return -1;
}
