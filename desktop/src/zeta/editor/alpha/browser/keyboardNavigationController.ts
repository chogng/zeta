import { addDisposableListener } from "../../../base/browser/dom.js";
import { StandardKeyboardEvent } from "../../../base/browser/keyboardEvent.js";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
import { operatingSystem, OperatingSystem } from "../../../base/common/platform.js";
import { EditorCursorNavigationCommand, EditorCursorNavigationMode, navigateEditorCursors } from "../common/cursorNavigation.js";
import { type EditorSelectionController } from "../common/editorSelectionController.js";
import { type AlphaEditorViewport } from "./alphaEditorViewport.js";

export interface AlphaKeyboardNavigationControllerOptions {
  readonly operatingSystem?: OperatingSystem;
}

export interface AlphaKeyboardNavigationCommand {
  readonly command: EditorCursorNavigationCommand;
  readonly mode: EditorCursorNavigationMode;
}

/**
 * Routes browser keydown navigation into Alpha common selection commands.
 */
export class AlphaKeyboardNavigationController extends DisposableOwner {
  private readonly targetOperatingSystem: OperatingSystem;
  private preferredColumns: readonly number[] | undefined;
  private applyingNavigation = false;

  constructor(
    private readonly viewport: AlphaEditorViewport,
    private readonly selectionController: EditorSelectionController,
    options: AlphaKeyboardNavigationControllerOptions = {},
  ) {
    super();
    try {
      this.targetOperatingSystem = readOperatingSystem(
        options.operatingSystem,
      );
    } catch (error) {
      this.dispose();
      throw error;
    }
    if (viewport.textModel !== selectionController.textModel) {
      this.dispose();
      throw new TypeError(
        "Alpha keyboard and selection controllers must share one text model",
      );
    }
    this.own(addDisposableListener(
      viewport.element,
      "keydown",
      event => this.handleKeydown(event),
    ));
    this.own(selectionController.onDidChange(() => {
      if (!this.applyingNavigation) this.preferredColumns = undefined;
    }));
  }

  private handleKeydown(browserEvent: KeyboardEvent): void {
    if (browserEvent.defaultPrevented) return;
    const event = new StandardKeyboardEvent(browserEvent);
    const navigation = resolveAlphaKeyboardNavigation(
      event,
      this.targetOperatingSystem,
    );
    if (!navigation) return;
    event.stop();
    const layout = this.viewport.viewportLayout;
    const result = navigateEditorCursors(
      this.viewport.textModel,
      this.selectionController.selections,
      {
        ...navigation,
        pageLineCount: Math.max(
          1,
          Math.floor(
            layout.viewportSize.height /
            layout.lineHeight,
          ),
        ),
        preferredColumns: this.preferredColumns,
      },
    );
    this.applyingNavigation = true;
    try {
      this.selectionController.setSelections(result.selections);
    } finally {
      this.applyingNavigation = false;
    }
    this.preferredColumns = result.preferredColumns;
    this.viewport.revealPosition(result.selections.primary.active);
  }
}

export function resolveAlphaKeyboardNavigation(event: Pick<StandardKeyboardEvent, "key" | "ctrlKey" | "shiftKey" | "altKey" | "metaKey" | "altGraphKey" | "isComposing">, targetOperatingSystem: OperatingSystem): AlphaKeyboardNavigationCommand | undefined {
  if (event.isComposing || event.altGraphKey) return undefined;
  const mode = event.shiftKey
    ? EditorCursorNavigationMode.Extend
    : EditorCursorNavigationMode.Move;
  const noCommandModifier =
    !event.ctrlKey && !event.altKey && !event.metaKey;
  if (noCommandModifier) {
    const command = unmodifiedCommand(event.key);
    return command ? { command, mode } : undefined;
  }

  if (targetOperatingSystem === OperatingSystem.Macintosh) {
    if (event.altKey && !event.ctrlKey && !event.metaKey) {
      if (event.key === "ArrowLeft") {
        return { command: EditorCursorNavigationCommand.WordLeft, mode };
      }
      if (event.key === "ArrowRight") {
        return { command: EditorCursorNavigationCommand.WordRight, mode };
      }
    }
    if (event.metaKey && !event.ctrlKey && !event.altKey) {
      const command = macCommandCommand(event.key);
      return command ? { command, mode } : undefined;
    }
    return undefined;
  }

  if (event.ctrlKey && !event.altKey && !event.metaKey) {
    const command = controlCommand(event.key);
    return command ? { command, mode } : undefined;
  }
  return undefined;
}

function unmodifiedCommand(key: string): EditorCursorNavigationCommand | undefined {
  switch (key) {
    case "ArrowLeft":
      return EditorCursorNavigationCommand.CharacterLeft;
    case "ArrowRight":
      return EditorCursorNavigationCommand.CharacterRight;
    case "ArrowUp":
      return EditorCursorNavigationCommand.LineUp;
    case "ArrowDown":
      return EditorCursorNavigationCommand.LineDown;
    case "Home":
      return EditorCursorNavigationCommand.LineStart;
    case "End":
      return EditorCursorNavigationCommand.LineEnd;
    case "PageUp":
      return EditorCursorNavigationCommand.PageUp;
    case "PageDown":
      return EditorCursorNavigationCommand.PageDown;
    default:
      return undefined;
  }
}

function controlCommand(key: string): EditorCursorNavigationCommand | undefined {
  switch (key) {
    case "ArrowLeft":
      return EditorCursorNavigationCommand.WordLeft;
    case "ArrowRight":
      return EditorCursorNavigationCommand.WordRight;
    case "Home":
      return EditorCursorNavigationCommand.DocumentStart;
    case "End":
      return EditorCursorNavigationCommand.DocumentEnd;
    default:
      return undefined;
  }
}

function macCommandCommand(key: string): EditorCursorNavigationCommand | undefined {
  switch (key) {
    case "ArrowLeft":
      return EditorCursorNavigationCommand.LineStart;
    case "ArrowRight":
      return EditorCursorNavigationCommand.LineEnd;
    case "ArrowUp":
    case "Home":
      return EditorCursorNavigationCommand.DocumentStart;
    case "ArrowDown":
    case "End":
      return EditorCursorNavigationCommand.DocumentEnd;
    default:
      return undefined;
  }
}

function readOperatingSystem(value: OperatingSystem | undefined): OperatingSystem {
  const resolved = value ?? operatingSystem;
  if (!Object.values(OperatingSystem).includes(resolved)) {
    throw new TypeError("Unknown Alpha keyboard operating system");
  }
  return resolved;
}
