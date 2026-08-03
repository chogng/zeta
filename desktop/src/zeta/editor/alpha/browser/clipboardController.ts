import { addDisposableListener, stopEvent } from "../../../base/browser/dom.js";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
import { isWindows } from "../../../base/common/platform.js";
import { EditorClipboardPasteMode, EditorEmptySelectionClipboardPolicy, getEditorClipboardEntries, type EditorClipboardEntry } from "../common/clipboard.js";
import { createClipboardCutCommand, createDistributedPasteTextCommand, createLinePasteCommand, createPasteTextCommand } from "../common/editCommands.js";
import { type EditorEditCommand, type EditorSelectionController } from "../common/editorSelectionController.js";
import { type TextSelectionSet } from "../common/selection.js";
import { type TextModel } from "../common/textModel.js";
import { type AlphaEditorViewport } from "./alphaEditorViewport.js";
import { type AlphaSemanticTokenSource } from "../language/browser/semanticTokenPresentation.js";
import { createAlphaSyntaxClipboardHtml } from "./syntaxClipboardHtml.js";
import { ALPHA_TEXT_FILE_TRANSFER_MAX_BYTES, selectAlphaTextFileTransfer } from "./textFileTransfer.js";
import { captureAlphaClipboardTextTransfer, normalizeAlphaClipboardPasteProviders, provideAlphaClipboardPaste, type AlphaClipboardPasteProvider } from "./clipboardPasteProvider.js";
import { createAlphaBrowserClipboardSystemTextReader, type AlphaClipboardSystemTextReader } from "./clipboardSystemText.js";
import { createAlphaBrowserClipboardRichTextReader, createAlphaBrowserClipboardRichTextWriter, type AlphaClipboardRichTextItem, type AlphaClipboardRichTextReader, type AlphaClipboardRichTextWriter } from "./clipboardRichText.js";

export const ALPHA_EDITOR_CLIPBOARD_MIME = "application/x-zeta-alpha-editor";
export const ALPHA_EDITOR_HTML_CLIPBOARD_MIME = "text/html";

export enum AlphaClipboardLineEnding {
  LF = "\n",
  CRLF = "\r\n",
}

export interface AlphaClipboardControllerOptions {
  readonly lineEnding?: AlphaClipboardLineEnding;
  readonly emptySelectionPolicy?: EditorEmptySelectionClipboardPolicy;
  /** Optional current token projection used only for portable HTML copy output. */
  readonly semanticTokens?: AlphaSemanticTokenSource;
  /** Rejects cut and paste while another input adapter owns a protected edit. */
  readonly isEditingAllowed?: () => boolean;
  /** Ordered local providers for declared non-plain clipboard representations. */
  readonly pasteProviders?: readonly AlphaClipboardPasteProvider[];
  /**
   * Optional Async Clipboard plain-text fallback. It is used only when the
   * native paste event has no textual, metadata, file, or provider payload.
   */
  readonly systemTextReader?: AlphaClipboardSystemTextReader;
  /** Optional rich Async Clipboard fallback, used before the plain-text fallback. */
  readonly richTextReader?: AlphaClipboardRichTextReader;
  /** Optional rich Async Clipboard writer, used only without event clipboard data. */
  readonly richTextWriter?: AlphaClipboardRichTextWriter;
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
  private readonly semanticTokens: AlphaSemanticTokenSource | undefined;
  private readonly isEditingAllowed: () => boolean;
  private readonly pasteProviders: readonly AlphaClipboardPasteProvider[];
  private readonly systemTextReader: AlphaClipboardSystemTextReader | undefined;
  private readonly richTextReader: AlphaClipboardRichTextReader | undefined;
  private readonly richTextWriter: AlphaClipboardRichTextWriter | undefined;
  private asynchronousPasteRequest = 0;
  private disposed = false;

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
    if (options.semanticTokens && options.semanticTokens.textModel !== viewport.textModel) {
      this.dispose();
      throw new TypeError("Alpha clipboard semantic tokens must share the viewport text model");
    }
    if (options.isEditingAllowed !== undefined && typeof options.isEditingAllowed !== "function") {
      this.dispose();
      throw new TypeError("Alpha clipboard edit gate must be a function");
    }
    if (options.systemTextReader !== undefined && typeof options.systemTextReader.readText !== "function") {
      this.dispose();
      throw new TypeError("Alpha clipboard system text reader must provide readText");
    }
    if (options.richTextReader !== undefined && typeof options.richTextReader.readText !== "function") {
      this.dispose();
      throw new TypeError("Alpha clipboard rich text reader must provide readText");
    }
    if (options.richTextWriter !== undefined && typeof options.richTextWriter.writeText !== "function") {
      this.dispose();
      throw new TypeError("Alpha clipboard rich text writer must provide writeText");
    }
    this.lineEnding = readLineEnding(options.lineEnding);
    this.emptySelectionPolicy = readEmptySelectionPolicy(
      options.emptySelectionPolicy,
    );
    this.semanticTokens = options.semanticTokens;
    this.isEditingAllowed = options.isEditingAllowed ?? (() => true);
    this.pasteProviders = normalizeAlphaClipboardPasteProviders(options.pasteProviders);
    this.systemTextReader = options.systemTextReader ?? createAlphaBrowserClipboardSystemTextReader(element.ownerDocument);
    this.richTextReader = options.richTextReader ?? createAlphaBrowserClipboardRichTextReader(element.ownerDocument);
    this.richTextWriter = options.richTextWriter ?? createAlphaBrowserClipboardRichTextWriter(element.ownerDocument);
    this.defer(() => {
      this.disposed = true;
      this.asynchronousPasteRequest += 1;
    });
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
    if (this.writeClipboard(event.clipboardData, entries)) {
      stopEvent(event);
      return;
    }
    this.writeRichSystemClipboard(event, entries);
  }

  private handleCut(event: ClipboardEvent): void {
    if (event.defaultPrevented) return;
    if (!this.isEditingAllowed()) {
      stopEvent(event);
      return;
    }
    const entries = getEditorClipboardEntries(
      this.viewport.textModel,
      this.selectionController.selections,
      this.emptySelectionPolicy,
    );
    if (this.writeClipboard(event.clipboardData, entries)) {
      stopEvent(event);
      this.executeCut();
      return;
    }
    this.writeRichSystemClipboard(event, entries, true);
  }

  private handlePaste(event: ClipboardEvent): void {
    const nativeClipboard = event.clipboardData;
    if (event.defaultPrevented || !nativeClipboard) return;
    if (!this.isEditingAllowed()) {
      stopEvent(event);
      return;
    }
    const text = readClipboardText(nativeClipboard, this.element.ownerDocument);
    const clipboardData = readClipboardMetadata(
      nativeClipboard,
      this.selectionController.selections.selections.length,
    );
    if (text.length === 0 && !clipboardData?.texts.some(value => value.length > 0)) {
      if (this.pasteProviders.some(provider => provider.mimeTypes.some(type => Array.from(nativeClipboard.types).includes(type)))) {
        this.pasteProvidedText(event);
        return;
      }
      if (this.pasteTextFile(event)) return;
      if (this.pasteRichSystemText(event)) return;
      this.pasteSystemText(event);
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

  private pasteTextFile(event: ClipboardEvent): boolean {
    const file = selectAlphaTextFileTransfer(event.clipboardData?.files ?? []);
    if (!file) return false;
    const model = this.viewport.textModel;
    const expectedVersion = model.version;
    const expectedSelections = this.selectionController.selections;
    const request = ++this.asynchronousPasteRequest;
    stopEvent(event);
    void file.text().then(text => {
      if (
        this.disposed ||
        request !== this.asynchronousPasteRequest ||
        text.length > ALPHA_TEXT_FILE_TRANSFER_MAX_BYTES ||
        !this.isEditingAllowed() ||
        model.version !== expectedVersion ||
        !selectionSetsEqual(this.selectionController.selections, expectedSelections)
      ) {
        return;
      }
      this.selectionController.execute(createPasteTextCommand(model, expectedSelections, text));
      this.afterEdit();
    }).catch(() => {
      // The host supplied the file, but it could not be decoded as text.
    });
    return true;
  }

  private pasteProvidedText(event: ClipboardEvent): void {
    const clipboardData = event.clipboardData;
    if (!clipboardData) return;
    const model = this.viewport.textModel;
    const expectedVersion = model.version;
    const expectedSelections = this.selectionController.selections;
    const request = ++this.asynchronousPasteRequest;
    const transfer = captureAlphaClipboardTextTransfer(clipboardData);
    stopEvent(event);
    void provideAlphaClipboardPaste(this.pasteProviders, transfer).then(text => {
      if (
        text === undefined ||
        this.disposed ||
        request !== this.asynchronousPasteRequest ||
        !this.isEditingAllowed() ||
        model.version !== expectedVersion ||
        !selectionSetsEqual(this.selectionController.selections, expectedSelections)
      ) {
        return;
      }
      this.selectionController.execute(createPasteTextCommand(model, expectedSelections, text));
      this.afterEdit();
    }).catch(() => {
      // A provider is optional; invalid or failed output must not mutate the model.
    });
  }

  private pasteSystemText(event: ClipboardEvent): boolean {
    const reader = this.systemTextReader;
    if (!reader) return false;
    const model = this.viewport.textModel;
    const expectedVersion = model.version;
    const expectedSelections = this.selectionController.selections;
    const request = ++this.asynchronousPasteRequest;
    stopEvent(event);
    void Promise.resolve(reader.readText()).then(text => {
      if (
        text.length === 0 ||
        this.disposed ||
        request !== this.asynchronousPasteRequest ||
        !this.isEditingAllowed() ||
        model.version !== expectedVersion ||
        !selectionSetsEqual(this.selectionController.selections, expectedSelections)
      ) {
        return;
      }
      this.selectionController.execute(createPasteTextCommand(model, expectedSelections, text));
      this.afterEdit();
    }).catch(() => {
      // Permission failures and unavailable system text must leave the model unchanged.
    });
    return true;
  }

  private pasteRichSystemText(event: ClipboardEvent): boolean {
    const reader = this.richTextReader;
    if (!reader) return false;
    const model = this.viewport.textModel;
    const expectedVersion = model.version;
    const expectedSelections = this.selectionController.selections;
    const request = ++this.asynchronousPasteRequest;
    stopEvent(event);
    void Promise.resolve(reader.readText()).then(item => {
      const text = item?.plainText ?? (item?.html ? readAlphaHtmlText(item.html, this.element.ownerDocument) : "");
      if (text.length === 0 || this.disposed || request !== this.asynchronousPasteRequest || !this.isEditingAllowed() || model.version !== expectedVersion || !selectionSetsEqual(this.selectionController.selections, expectedSelections)) return;
      this.selectionController.execute(createPasteTextCommand(model, expectedSelections, text));
      this.afterEdit();
    }).catch(() => {
      // Permission and representation failures leave the model unchanged.
    });
    return true;
  }

  private writeRichSystemClipboard(event: ClipboardEvent, entries: readonly EditorClipboardEntry[], cut = false): boolean {
    const writer = this.richTextWriter;
    if (!writer || !entries.some(entry => entry.text.length > 0)) return false;
    const model = this.viewport.textModel;
    const expectedVersion = model.version;
    const expectedSelections = this.selectionController.selections;
    const request = ++this.asynchronousPasteRequest;
    const payload = this.createClipboardPayload(entries);
    stopEvent(event);
    void Promise.resolve(writer.writeText(payload)).then(() => {
      if (!cut || this.disposed || request !== this.asynchronousPasteRequest || !this.isEditingAllowed() || model.version !== expectedVersion || !selectionSetsEqual(this.selectionController.selections, expectedSelections)) return;
      this.executeCut();
    }).catch(() => {
      // Permission failures must never mutate the model, especially for cut.
    });
    return true;
  }

  private executeCut(): void {
    this.selectionController.execute(createClipboardCutCommand(
      this.viewport.textModel,
      this.selectionController.selections,
      this.emptySelectionPolicy,
    ));
    this.afterEdit();
  }

  private writeClipboard(clipboardData: DataTransfer | null, entries: readonly EditorClipboardEntry[]): boolean {
    if (!clipboardData) return false;
    if (!entries.some(entry => entry.text.length > 0)) return false;
    const payload = this.createClipboardPayload(entries);
    try {
      clipboardData.setData(
        "text/plain",
        payload.plainText,
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
    try {
      clipboardData.setData(ALPHA_EDITOR_HTML_CLIPBOARD_MIME, payload.html);
    } catch {
      // Plain text remains authoritative when a browser rejects HTML clipboard data.
    }
    return true;
  }

  private createClipboardPayload(entries: readonly EditorClipboardEntry[]): Required<AlphaClipboardRichTextItem> {
    return Object.freeze({
      plainText: joinClipboardEntries(entries, this.lineEnding),
      html: createAlphaSyntaxClipboardHtml(
        entries,
        this.lineEnding,
        this.semanticTokens,
        this.element.ownerDocument,
      ),
    });
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

function readClipboardText(clipboardData: DataTransfer, ownerDocument: Document): string {
  try {
    const text = clipboardData.getData("text/plain");
    if (text.length > 0) return text;
  } catch {
    // A browser may expose only a rich clipboard representation.
  }
  try {
    return readAlphaHtmlText(clipboardData.getData(ALPHA_EDITOR_HTML_CLIPBOARD_MIME), ownerDocument);
  } catch {
    return "";
  }
}

function selectionSetsEqual(left: TextSelectionSet, right: TextSelectionSet): boolean {
  return left.primaryIndex === right.primaryIndex &&
    left.selections.length === right.selections.length &&
    left.selections.every((selection, index) => {
      const expected = right.selections[index]!;
      return selection.anchor.compareTo(expected.anchor) === 0 &&
        selection.active.compareTo(expected.active) === 0;
    });
}

/** Reduces untrusted HTML to inert deterministic text for Alpha paste and drop paths. */
export function readAlphaHtmlText(html: string, ownerDocument: Document): string {
  if (html.length === 0) return "";
  const template = ownerDocument.createElement("template");
  template.innerHTML = html;
  const parts: string[] = [];
  appendHtmlClipboardText(template.content, parts);
  return parts.join("").replaceAll("\u00a0", " ").replace(/\n{3,}/g, "\n\n").replace(/^\n|\n$/g, "");
}

function appendHtmlClipboardText(node: Node, parts: string[]): void {
  if (node.nodeType === node.TEXT_NODE) {
    parts.push(node.textContent ?? "");
    return;
  }
  if (node.nodeType !== node.ELEMENT_NODE && node.nodeType !== node.DOCUMENT_FRAGMENT_NODE) return;
  const element = node.nodeType === node.ELEMENT_NODE ? node as HTMLElement : undefined;
  if (element && (element.localName === "script" || element.localName === "style" || element.localName === "noscript")) return;
  if (element?.localName === "br") {
    appendLineBreak(parts);
    return;
  }
  const block = element !== undefined && HTML_CLIPBOARD_BLOCK_ELEMENTS.has(element.localName);
  if (block) appendLineBreak(parts);
  for (const child of node.childNodes) appendHtmlClipboardText(child, parts);
  if (block) appendLineBreak(parts);
}

function appendLineBreak(parts: string[]): void {
  if (parts.length === 0 || parts.at(-1) !== "\n") parts.push("\n");
}

const HTML_CLIPBOARD_BLOCK_ELEMENTS = new Set([
  "address", "article", "aside", "blockquote", "div", "dl", "dt", "dd", "fieldset", "figcaption",
  "figure", "footer", "form", "h1", "h2", "h3", "h4", "h5", "h6", "header", "hr", "li",
  "main", "nav", "ol", "p", "section", "table", "tbody", "td", "tfoot", "th", "thead", "tr", "ul",
]);

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
