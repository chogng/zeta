import { addDisposableListener, stopEvent } from "../../../base/browser/dom.js";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
import { operatingSystem, OperatingSystem } from "../../../base/common/platform.js";
import { type EditorSelectionController } from "../common/editorSelectionController.js";
import { createDeleteLinesCommand, createDuplicateLinesCommand, createInsertLineCommand, createMoveLinesCommand, EditorLineDuplicateDirection, EditorLineInsertDirection, EditorLineMoveDirection } from "../common/lineOperations.js";
import { type AlphaEditorViewport } from "./alphaEditorViewport.js";

export interface AlphaLineOperationsControllerOptions {
  readonly operatingSystem?: OperatingSystem;
}

/** Routes VS Code-compatible delete, duplicate, and move-line chords locally. */
export class AlphaLineOperationsController extends DisposableOwner {
  private readonly targetOperatingSystem: OperatingSystem;

  constructor(
    input: HTMLTextAreaElement,
    private readonly viewport: AlphaEditorViewport,
    private readonly selections: EditorSelectionController,
    options: AlphaLineOperationsControllerOptions = {},
  ) {
    super();
    try {
      this.targetOperatingSystem = readOperatingSystem(options.operatingSystem);
      if (viewport.textModel !== selections.textModel) {
        throw new TypeError("Alpha line operation dependencies must share one text model");
      }
      this.own(addDisposableListener(input, "keydown", event => this.handleKeydown(event)));
    } catch (error) {
      this.dispose();
      throw error;
    }
  }

  private handleKeydown(event: KeyboardEvent): void {
    if (event.defaultPrevented || event.isComposing || event.getModifierState("AltGraph")) return;
    if ((event.ctrlKey || event.metaKey) && event.shiftKey && !event.altKey && event.key.toLowerCase() === "k") {
      stopEvent(event);
      this.selections.execute(createDeleteLinesCommand(
        this.viewport.textModel,
        this.selections.selections,
      ));
      this.viewport.revealPosition(this.selections.selections.primary.active);
      return;
    }
    if ((event.ctrlKey || event.metaKey) && !event.altKey && event.key === "Enter") {
      stopEvent(event);
      this.selections.execute(createInsertLineCommand(
        this.viewport.textModel,
        this.selections.selections,
        event.shiftKey ? EditorLineInsertDirection.Before : EditorLineInsertDirection.After,
      ));
      this.viewport.revealPosition(this.selections.selections.primary.active);
      return;
    }
    if (!event.altKey) return;
    if (!event.shiftKey) {
      if (event.ctrlKey || event.metaKey) return;
      const moveDirection = event.key === "ArrowUp"
        ? EditorLineMoveDirection.Up
        : event.key === "ArrowDown"
          ? EditorLineMoveDirection.Down
          : undefined;
      if (!moveDirection) return;
      stopEvent(event);
      this.selections.execute(createMoveLinesCommand(
        this.viewport.textModel,
        this.selections.selections,
        moveDirection,
      ));
      this.viewport.revealPosition(this.selections.selections.primary.active);
      return;
    }
    const duplicateDirection = resolveAlphaDuplicateLineDirection(event, this.targetOperatingSystem);
    if (!duplicateDirection) return;
    stopEvent(event);
    this.selections.execute(createDuplicateLinesCommand(
      this.viewport.textModel,
      this.selections.selections,
      duplicateDirection,
    ));
    this.viewport.revealPosition(this.selections.selections.primary.active);
  }
}

/** Resolves a duplicate-line chord without colliding with the platform multi-cursor binding. */
export function resolveAlphaDuplicateLineDirection(event: Pick<KeyboardEvent, "key" | "ctrlKey" | "shiftKey" | "altKey" | "metaKey">, targetOperatingSystem: OperatingSystem): EditorLineDuplicateDirection | undefined {
  const direction = event.key === "ArrowUp"
    ? EditorLineDuplicateDirection.Up
    : event.key === "ArrowDown"
      ? EditorLineDuplicateDirection.Down
      : undefined;
  if (!direction || !event.altKey || !event.shiftKey || event.metaKey) return undefined;
  if (targetOperatingSystem === OperatingSystem.Linux) {
    return event.ctrlKey ? direction : undefined;
  }
  return event.ctrlKey ? undefined : direction;
}

function readOperatingSystem(value: OperatingSystem | undefined): OperatingSystem {
  const resolved = value ?? operatingSystem;
  if (!Object.values(OperatingSystem).includes(resolved)) {
    throw new TypeError("Unknown Alpha line operation operating system");
  }
  return resolved;
}
