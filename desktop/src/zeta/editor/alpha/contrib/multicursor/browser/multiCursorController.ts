import { addDisposableListener, stopEvent } from "../../../../../base/browser/dom.js";
import { DisposableOwner } from "../../../../../base/common/lifecycle.js";
import { operatingSystem, OperatingSystem } from "../../../../../base/common/platform.js";
import { addAdjacentLineCursors, addCursorsToSelectedLineEnds, EditorCursorInsertionDirection } from "../../../common/cursor/cursorInsertion.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { type EditorViewport } from "../../../browser/view/editorViewport.js";

export interface MultiCursorControllerOptions {
  readonly operatingSystem?: OperatingSystem;
}

/** Routes platform-specific add-cursor-above/below chords through Alpha common state. */
export class MultiCursorController extends DisposableOwner {
  private readonly targetOperatingSystem: OperatingSystem;

  constructor(
    input: HTMLTextAreaElement,
    private readonly viewport: EditorViewport,
    private readonly selections: EditorSelectionController,
    options: MultiCursorControllerOptions = {},
  ) {
    super();
    try {
      this.targetOperatingSystem = readOperatingSystem(options.operatingSystem);
      if (viewport.textModel !== selections.textModel) {
        throw new TypeError("Alpha multi-cursor dependencies must share one text model");
      }
      this.own(addDisposableListener(input, "keydown", event => this.handleKeydown(event)));
    } catch (error) {
      this.dispose();
      throw error;
    }
  }

  private handleKeydown(event: KeyboardEvent): void {
    if (event.defaultPrevented || event.isComposing || event.getModifierState("AltGraph")) return;
    if (event.shiftKey && event.altKey && !event.ctrlKey && !event.metaKey && event.key.toLowerCase() === "i") {
      const next = addCursorsToSelectedLineEnds(this.viewport.textModel, this.selections.selections);
      if (next === this.selections.selections) return;
      stopEvent(event);
      this.selections.setCursorSelections(next);
      this.viewport.revealPosition(next.primary.active);
      return;
    }
    const direction = resolveAlphaAdjacentCursorDirection(event, this.targetOperatingSystem);
    if (!direction) return;
    stopEvent(event);
    const next = addAdjacentLineCursors(this.viewport.textModel, this.selections.selections, direction);
    this.selections.setCursorSelections(next);
    this.viewport.revealPosition(next.primary.active);
  }
}

/** Resolves the non-conflicting VS Code add-cursor chord for a host platform. */
export function resolveAlphaAdjacentCursorDirection(event: Pick<KeyboardEvent, "key" | "ctrlKey" | "shiftKey" | "altKey" | "metaKey">, targetOperatingSystem: OperatingSystem): EditorCursorInsertionDirection | undefined {
  const direction = event.key === "ArrowUp"
    ? EditorCursorInsertionDirection.Above
    : event.key === "ArrowDown"
      ? EditorCursorInsertionDirection.Below
      : undefined;
  if (!direction) return undefined;
  if (targetOperatingSystem === OperatingSystem.Macintosh) {
    return event.metaKey && event.altKey && !event.ctrlKey && !event.shiftKey ? direction : undefined;
  }
  if (targetOperatingSystem === OperatingSystem.Windows) {
    return event.ctrlKey && event.altKey && !event.metaKey && !event.shiftKey ? direction : undefined;
  }
  return event.ctrlKey && event.shiftKey && event.altKey && !event.metaKey ? direction : undefined;
}

function readOperatingSystem(value: OperatingSystem | undefined): OperatingSystem {
  const resolved = value ?? operatingSystem;
  if (!Object.values(OperatingSystem).includes(resolved)) {
    throw new TypeError("Unknown Alpha multi-cursor operating system");
  }
  return resolved;
}
