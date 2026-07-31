import { addDisposableListener, stopEvent } from "../../../base/browser/dom.js";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
import { isWindows } from "../../../base/common/platform.js";
import { EditorClipboardPasteMode, EditorEmptySelectionClipboardPolicy, getEditorClipboardEntries, type EditorClipboardEntry } from "../common/clipboard.js";
import { createClipboardCutCommand, createDistributedPasteTextCommand, createLinePasteCommand, createPasteTextCommand } from "../common/editCommands.js";
import { type EditorEditCommand, type EditorSelectionController } from "../common/editorSelectionController.js";
import { type TextSelectionSet } from "../common/selection.js";
import { type TextModel } from "../common/textModel.js";
import { type AlphaEditorViewport } from "./alphaEditorViewport.js";

export const ALPHA_EDITOR_CLIPBOARD_MIME = "application/x-zeta-alpha-editor";

export enum AlphaClipboardLineEnding {
  LF = "\n",
  CRLF = "\r\n",
}

export interface AlphaClipboardControllerOptions {
  readonly lineEnding?: AlphaClipboardLineEnding;
  readonly emptySelectionPolicy?: EditorEmptySelectionClipboardPolicy;
}

interface AlphaClipboardMetadata {
  readonly version: 2;
  readonly selectionTexts: readonly string[];
  readonly pasteModes: readonly EditorClipboardPasteMode[];
}

interface AlphaClipboardPasteData {
  readonly texts: readonly string[];
  readonly modes: readonly EditorClipboardPasteMode[];
}

/**
 * Routes native clipboard events through Alpha's selection-aware commands.
 */
export class AlphaClipboardController extends DisposableOwner {
  private readonly lineEnding: AlphaClipboardLineEnding;
  private readonly emptySelectionPolicy: EditorEmptySelectionClipboardPolicy;

  constructor(
    private readonly element: HTMLTextAreaElement,
    private readonly viewport: AlphaEditorViewport,
    private readonly selectionController: EditorSelectionController,
    options: AlphaClipboardControllerOptions = {},
  ) {
    super();
    if (viewport.textModel !== selectionController.textModel) {
      this.dispose();
      throw new TypeError(
        "Alpha clipboard and selection controllers must share one text model",
      );
    }
    this.lineEnding = readLineEnding(options.lineEnding);
    this.emptySelectionPolicy = readEmptySelectionPolicy(
      options.emptySelectionPolicy,
    );
    this.own(addDisposableListener<ClipboardEvent>(
      element,
      "copy",
      event => this.handleCopy(event),
    ));
    this.own(addDisposableListener<ClipboardEvent>(
      element,
      "cut",
      event => this.handleCut(event),
    ));
    this.own(addDisposableListener<ClipboardEvent>(
      element,
      "paste",
      event => this.handlePaste(event),
    ));
  }

  private handleCopy(event: ClipboardEvent): void {
    if (event.defaultPrevented) return;
    const entries = getEditorClipboardEntries(
      this.viewport.textModel,
      this.selectionController.selections,
      this.emptySelectionPolicy,
    );
    if (!this.writeClipboard(event.clipboardData, entries)) return;
    stopEvent(event);
  }

  private handleCut(event: ClipboardEvent): void {
    if (event.defaultPrevented) return;
    const entries = getEditorClipboardEntries(
      this.viewport.textModel,
      this.selectionController.selections,
      this.emptySelectionPolicy,
    );
    if (!this.writeClipboard(event.clipboardData, entries)) return;
    stopEvent(event);
    this.selectionController.execute(createClipboardCutCommand(
      this.viewport.textModel,
      this.selectionController.selections,
      this.emptySelectionPolicy,
    ));
    this.afterEdit();
  }

  private handlePaste(event: ClipboardEvent): void {
    if (event.defaultPrevented || !event.clipboardData) return;
    const text = readClipboardText(event.clipboardData);
    const clipboardData = readClipboardMetadata(
      event.clipboardData,
      this.selectionController.selections.selections.length,
    );
    if (
      text.length === 0 &&
      !clipboardData?.texts.some(value => value.length > 0)
    ) {
      return;
    }
    const command = clipboardData
      ? createMetadataPasteCommand(
        this.viewport.textModel,
        this.selectionController.selections,
        clipboardData,
      )
      : createPasteTextCommand(
        this.viewport.textModel,
        this.selectionController.selections,
        text,
      );
    stopEvent(event);
    this.selectionController.execute(command);
    this.afterEdit();
  }

  private writeClipboard(clipboardData: DataTransfer | null, entries: readonly EditorClipboardEntry[]): boolean {
    if (!clipboardData) return false;
    if (!entries.some(entry => entry.text.length > 0)) return false;
    try {
      clipboardData.setData(
        "text/plain",
        joinClipboardEntries(entries, this.lineEnding),
      );
    } catch {
      return false;
    }
    const metadata: AlphaClipboardMetadata = {
      version: 2,
      selectionTexts: entries.map(entry => entry.text),
      pasteModes: entries.map(entry => entry.pasteMode),
    };
    try {
      clipboardData.setData(
        ALPHA_EDITOR_CLIPBOARD_MIME,
        JSON.stringify(metadata),
      );
    } catch {
      // Plain text remains portable when a browser rejects custom MIME data.
    }
    return true;
  }

  private afterEdit(): void {
    this.element.value = "";
    this.viewport.revealPosition(
      this.selectionController.selections.primary.active,
    );
  }
}

function createMetadataPasteCommand(model: TextModel, selections: TextSelectionSet, data: AlphaClipboardPasteData): EditorEditCommand {
  return data.modes.every(mode => mode === EditorClipboardPasteMode.Line) &&
    canPasteCompleteLines(selections)
    ? createLinePasteCommand(model, selections, data.texts)
    : createDistributedPasteTextCommand(model, selections, data.texts);
}

function canPasteCompleteLines(selections: TextSelectionSet): boolean {
  return selections.selections.every(selection => selection.collapsed);
}

function readLineEnding(lineEnding: AlphaClipboardLineEnding | undefined): AlphaClipboardLineEnding {
  const resolved = lineEnding ?? (
    isWindows ? AlphaClipboardLineEnding.CRLF : AlphaClipboardLineEnding.LF
  );
  if (!Object.values(AlphaClipboardLineEnding).includes(resolved)) {
    throw new TypeError("Unknown Alpha clipboard line ending");
  }
  return resolved;
}

function readEmptySelectionPolicy(policy: EditorEmptySelectionClipboardPolicy | undefined): EditorEmptySelectionClipboardPolicy {
  const resolved = policy ?? EditorEmptySelectionClipboardPolicy.Line;
  if (!Object.values(EditorEmptySelectionClipboardPolicy).includes(resolved)) {
    throw new TypeError("Unknown Alpha empty-selection clipboard policy");
  }
  return resolved;
}

function joinClipboardEntries(entries: readonly EditorClipboardEntry[], lineEnding: AlphaClipboardLineEnding): string {
  const included = entries.filter(entry => entry.text.length > 0);
  let result = "";
  let previousMode: EditorClipboardPasteMode | undefined;
  for (const entry of included) {
    if (
      result.length > 0 &&
      previousMode !== EditorClipboardPasteMode.Line
    ) {
      result += lineEnding;
    }
    result += toExternalLineEndings(entry.text, lineEnding);
    previousMode = entry.pasteMode;
  }
  return result;
}

function toExternalLineEndings(text: string, lineEnding: AlphaClipboardLineEnding): string {
  return lineEnding === AlphaClipboardLineEnding.LF
    ? text
    : text.replaceAll("\n", AlphaClipboardLineEnding.CRLF);
}

function readClipboardText(clipboardData: DataTransfer): string {
  try {
    return clipboardData.getData("text/plain");
  } catch {
    return "";
  }
}

function readClipboardMetadata(clipboardData: DataTransfer, selectionCount: number): AlphaClipboardPasteData | undefined {
  let parsed: unknown;
  try {
    const raw = clipboardData.getData(ALPHA_EDITOR_CLIPBOARD_MIME);
    if (!raw) return undefined;
    parsed = JSON.parse(raw);
  } catch {
    return undefined;
  }
  if (
    typeof parsed !== "object" ||
    parsed === null ||
    !("version" in parsed) ||
    (parsed.version !== 1 && parsed.version !== 2) ||
    !("selectionTexts" in parsed) ||
    !Array.isArray(parsed.selectionTexts) ||
    parsed.selectionTexts.length !== selectionCount ||
    parsed.selectionTexts.some(text => typeof text !== "string")
  ) {
    return undefined;
  }
  const texts = parsed.selectionTexts as string[];
  let modes = parsed.version === 2 &&
    "pasteModes" in parsed &&
    Array.isArray(parsed.pasteModes) &&
    parsed.pasteModes.length === selectionCount &&
    parsed.pasteModes.every(mode =>
      Object.values(EditorClipboardPasteMode).includes(mode)
    )
    ? parsed.pasteModes as EditorClipboardPasteMode[]
    : texts.map(() => EditorClipboardPasteMode.Selection);
  if (modes.some((mode, index) =>
    mode === EditorClipboardPasteMode.Line &&
    !texts[index]!.endsWith("\n")
  )) {
    modes = texts.map(() => EditorClipboardPasteMode.Selection);
  }
  return Object.freeze({
    texts: Object.freeze([...texts]),
    modes: Object.freeze([...modes]),
  });
}
