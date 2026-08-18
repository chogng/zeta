import "./diffEditorWidget.css";
import { addDisposableListener, fragment as createFragment, h, isHTMLElement, reset, stopEvent } from "../../../../base/browser/dom.js";
import { getClientArea, type IDimension } from "../../../../base/browser/geometry.js";
import { observeResize } from "../../../../base/browser/observer.js";
import { getWindow } from "../../../../base/browser/window.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { DiffModel } from "../../../common/diff/diffModel.js";
import { LineDiffKind, type DiffRange, type LineDiff, type LineDiffRow } from "../../../common/diff/lineDiff.js";

const DEFAULT_LINE_HEIGHT = 20;
const DEFAULT_OVERSCAN_ROW_COUNT = 8;

export interface DiffEditorWidgetOptions {
  readonly container: HTMLElement;
  readonly model: DiffModel;
  readonly lineHeight?: number;
  readonly fontFamily?: string;
  readonly fontSize?: number;
  readonly fontLigatures?: boolean;
  readonly showLineNumbers?: boolean;
  readonly showInlineChanges?: boolean;
  readonly loopChanges?: boolean;
  readonly overscanRowCount?: number;
  readonly originalAriaLabel?: string;
  readonly modifiedAriaLabel?: string;
}

/**
 * Read-only, virtualized side-by-side presentation of one DiffModel.
 *
 * The common model owns source versions, computation, correspondence, and
 * inline change ranges. This browser component owns only scroll geometry and
 * DOM projection; it never owns source text or diff computation.
 */
export class DiffEditorWidget extends DisposableOwner {
  readonly element: HTMLDivElement;
  private readonly contentElement: HTMLDivElement;
  private readonly rowsElement: HTMLDivElement;
  private readonly accessibilityStatusElement: HTMLDivElement;
  private readonly model: DiffModel;
  private readonly lineHeight: number;
  private readonly overscanRowCount: number;
  private currentDiff: LineDiff | undefined;
  private renderedStartRow = -1;
  private renderedEndRow = -1;
  private viewportHeight = 0;
  private activeChangeRow = -1;
  private readonly showInlineChanges: boolean;
  private readonly loopChanges: boolean;

  constructor(options: DiffEditorWidgetOptions) {
    super();
    validateOptions(options);
    const ownerDocument = options.container.ownerDocument;
    this.model = options.model;
    this.lineHeight = options.lineHeight ?? DEFAULT_LINE_HEIGHT;
    this.overscanRowCount = options.overscanRowCount ?? DEFAULT_OVERSCAN_ROW_COUNT;
    this.showInlineChanges = options.showInlineChanges ?? true;
    this.loopChanges = options.loopChanges ?? true;
    this.currentDiff = this.model.diff;
    this.element = h(ownerDocument, "div");
    this.contentElement = h(ownerDocument, "div");
    this.rowsElement = h(ownerDocument, "div");
    this.accessibilityStatusElement = h(ownerDocument, "div");
    this.element.className = "aster-diff-editor";
    this.element.classList.toggle("hide-line-numbers", options.showLineNumbers === false);
    if (options.fontFamily) this.element.style.fontFamily = options.fontFamily;
    if (options.fontSize !== undefined) this.element.style.fontSize = `${options.fontSize}px`;
    this.element.style.fontVariantLigatures = options.fontLigatures ? "normal" : "none";
    this.element.tabIndex = 0;
    this.element.setAttribute("role", "region");
    this.element.setAttribute("aria-label", `Side-by-side diff editor. Original: ${options.originalAriaLabel ?? "Original"}. Modified: ${options.modifiedAriaLabel ?? "Modified"}.`);
    this.contentElement.className = "aster-diff-editor-content";
    this.rowsElement.className = "aster-diff-editor-rows";
    this.accessibilityStatusElement.className = "aster-diff-editor-accessibility-status";
    this.accessibilityStatusElement.setAttribute("aria-live", "polite");
    this.accessibilityStatusElement.setAttribute("aria-atomic", "true");
    this.contentElement.append(this.rowsElement);
    this.element.append(this.contentElement, this.accessibilityStatusElement);
    options.container.append(this.element);
    this.defer(() => this.element.remove());
    this.own(addDisposableListener(this.element, "scroll", () => this.project()));
    this.own(addDisposableListener(this.element, "keydown", event => this.handleKeydown(event)));
    this.own(this.model.onDidChange(() => this.refresh()));
    this.own(observeResize(this.element, ([entry]) => {
      if (entry) this.layout({ width: entry.contentRect.width, height: entry.contentRect.height });
    }));
    this.layout(getClientArea(this.element));
  }

  get diff(): LineDiff | undefined {
    return this.currentDiff;
  }

  /** The currently revealed changed row, or -1 before change navigation starts. */
  get currentChangeRow(): number {
    return this.activeChangeRow;
  }

  layout(size: IDimension = getClientArea(this.element)): void {
    if (!Number.isFinite(size.width) || size.width < 0 || !Number.isFinite(size.height) || size.height < 0) {
      throw new RangeError("Diff editor widget layout size must be finite and non-negative");
    }
    this.viewportHeight = size.height;
    this.project(true);
  }

  private refresh(): void {
    this.currentDiff = this.model.diff;
    this.renderedStartRow = -1;
    this.renderedEndRow = -1;
    this.activeChangeRow = -1;
    this.accessibilityStatusElement.textContent = this.model.state.kind === "loading"
      ? "Computing differences"
      : this.model.state.kind === "error"
        ? `Could not compute differences: ${this.model.state.error.message}`
        : "";
    this.project(true);
  }

  revealOriginalLine(lineIndex: number): void {
    this.revealLine("originalLineIndex", lineIndex);
  }

  revealModifiedLine(lineIndex: number): void {
    this.revealLine("modifiedLineIndex", lineIndex);
  }

  /** Reveals the next changed row, wrapping to the first changed row when needed. */
  nextChange(): number | undefined {
    return this.selectRelativeChange(1);
  }

  /** Reveals the previous changed row, wrapping to the final changed row when needed. */
  previousChange(): number | undefined {
    return this.selectRelativeChange(-1);
  }

  private revealLine(side: "originalLineIndex" | "modifiedLineIndex", lineIndex: number): void {
    if (!Number.isSafeInteger(lineIndex) || lineIndex < 0) {
      throw new RangeError("Aster diff line index must be a non-negative safe integer");
    }
    const diff = this.currentDiff;
    if (!diff) throw new Error("Diff results are not ready");
    const rowIndex = diff.rows.findIndex(row => row[side] === lineIndex);
    if (rowIndex < 0) throw new RangeError("Aster diff line index is outside its source model");
    this.revealRow(rowIndex);
  }

  private selectRelativeChange(delta: -1 | 1): number | undefined {
    const diff = this.currentDiff;
    if (!diff) {
      this.accessibilityStatusElement.textContent = "Computing differences";
      return undefined;
    }
    const changedRows = diff.rows.flatMap((row, index) => row.kind === LineDiffKind.Unchanged ? [] : [index]);
    if (changedRows.length === 0) {
      this.accessibilityStatusElement.textContent = "No differences";
      return undefined;
    }
    const currentIndex = changedRows.indexOf(this.activeChangeRow);
    const selectedIndex = currentIndex < 0
      ? delta > 0 ? 0 : changedRows.length - 1
      : this.loopChanges
        ? (currentIndex + delta + changedRows.length) % changedRows.length
        : Math.max(0, Math.min(changedRows.length - 1, currentIndex + delta));
    const rowIndex = changedRows[selectedIndex]!;
    this.activeChangeRow = rowIndex;
    this.revealRow(rowIndex);
    const row = diff.rows[rowIndex]!;
    this.accessibilityStatusElement.textContent = `Change ${selectedIndex + 1} of ${changedRows.length}, ${diffRowLocation(row)}`;
    return rowIndex;
  }

  private revealRow(rowIndex: number): void {
    const rowTop = rowIndex * this.lineHeight;
    const rowBottom = rowTop + this.lineHeight;
    const viewportBottom = this.element.scrollTop + this.viewportHeight;
    if (rowTop < this.element.scrollTop) this.element.scrollTop = rowTop;
    else if (rowBottom > viewportBottom) this.element.scrollTop = Math.max(0, rowBottom - this.viewportHeight);
    this.project(true);
  }

  private handleKeydown(event: KeyboardEvent): void {
    if (event.defaultPrevented || event.isComposing || event.key !== "F7" || event.ctrlKey || event.metaKey || event.altKey) return;
    stopEvent(event);
    if (event.shiftKey) this.previousChange();
    else this.nextChange();
  }

  private project(force = false): void {
    const rows = this.currentDiff?.rows ?? [];
    this.contentElement.style.height = `${rows.length * this.lineHeight}px`;
    const visibleRowCount = Math.ceil(this.viewportHeight / this.lineHeight);
    const firstVisibleRow = Math.floor(this.element.scrollTop / this.lineHeight);
    const startRow = Math.max(0, firstVisibleRow - this.overscanRowCount);
    const endRow = Math.min(rows.length, firstVisibleRow + visibleRowCount + this.overscanRowCount);
    if (!force && startRow === this.renderedStartRow && endRow === this.renderedEndRow) return;
    const fragment = createFragment(this.element.ownerDocument);
    for (let rowIndex = startRow; rowIndex < endRow; rowIndex += 1) {
      const row = rows[rowIndex]!;
      fragment.append(createDiffRow(this.element.ownerDocument, row, this.model.original, this.model.modified, this.lineHeight, rowIndex === this.activeChangeRow, this.showInlineChanges));
    }
    this.rowsElement.style.transform = `translate3d(0, ${startRow * this.lineHeight}px, 0)`;
    reset(this.rowsElement, fragment);
    this.renderedStartRow = startRow;
    this.renderedEndRow = endRow;
  }
}

function createDiffRow(ownerDocument: Document, row: LineDiffRow, original: DiffModel["original"], modified: DiffModel["modified"], lineHeight: number, active: boolean, showInlineChanges: boolean): HTMLDivElement {
  const element = h(ownerDocument, "div");
  element.className = `aster-diff-editor-row ${row.kind}`;
  element.classList.toggle("active", active);
  element.style.height = `${lineHeight}px`;
  element.style.lineHeight = `${lineHeight}px`;
  element.append(
    createDiffCell(ownerDocument, "original", row.kind, row.originalLineIndex, row.originalLineIndex === undefined ? undefined : original.getLineContent(row.originalLineIndex), row.originalChanges, showInlineChanges),
    createDiffCell(ownerDocument, "modified", row.kind, row.modifiedLineIndex, row.modifiedLineIndex === undefined ? undefined : modified.getLineContent(row.modifiedLineIndex), row.modifiedChanges, showInlineChanges),
  );
  return element;
}

function diffRowLocation(row: LineDiffRow): string {
  const original = row.originalLineIndex === undefined ? "no original line" : `original line ${row.originalLineIndex + 1}`;
  const modified = row.modifiedLineIndex === undefined ? "no modified line" : `modified line ${row.modifiedLineIndex + 1}`;
  return `${original}, ${modified}`;
}

function createDiffCell(ownerDocument: Document, side: "original" | "modified", kind: LineDiffKind, lineIndex: number | undefined, text: string | undefined, changes: readonly DiffRange[], showInlineChanges: boolean): HTMLDivElement {
  const cell = h(ownerDocument, "div");
  cell.className = `aster-diff-editor-cell ${side}`;
  const number = h(ownerDocument, "span");
  number.className = "aster-diff-editor-line-number";
  number.textContent = lineIndex === undefined ? "" : String(lineIndex + 1);
  const content = h(ownerDocument, "span");
  content.className = "aster-diff-editor-line-content";
  if (text === undefined) {
    cell.classList.add("missing");
  } else {
    if (showInlineChanges) projectDiffText(ownerDocument, content, text, changes, side === "original" ? LineDiffKind.Removed : LineDiffKind.Added);
    else content.textContent = text;
    if (kind === LineDiffKind.Modified) cell.classList.add(side === "original" ? "removed" : "added");
    else if (kind === LineDiffKind.Removed && side === "original") cell.classList.add("removed");
    else if (kind === LineDiffKind.Added && side === "modified") cell.classList.add("added");
  }
  cell.append(number, content);
  return cell;
}

function projectDiffText(ownerDocument: Document, target: HTMLElement, text: string, changes: readonly DiffRange[], changedKind: LineDiffKind): void {
  const fragment = createFragment(ownerDocument);
  let previousEnd = 0;
  for (const change of changes) {
    fragment.append(text.slice(previousEnd, change.startColumn));
    const changed = h(ownerDocument, "span");
    changed.className = `aster-diff-editor-inline ${changedKind}`;
    changed.textContent = text.slice(change.startColumn, change.endColumn);
    fragment.append(changed);
    previousEnd = change.endColumn;
  }
  fragment.append(text.slice(previousEnd));
  reset(target, fragment);
}

function validateOptions(options: DiffEditorWidgetOptions): void {
  if (!options || typeof options !== "object" || !isHTMLElement(options.container)) {
    throw new TypeError("Diff editor widget requires a browser container");
  }
  if (!options.model || typeof options.model !== "object") {
    throw new TypeError("Diff editor widget requires a diff model");
  }
  const lineHeight = options.lineHeight ?? DEFAULT_LINE_HEIGHT;
  const overscanRowCount = options.overscanRowCount ?? DEFAULT_OVERSCAN_ROW_COUNT;
  if (!Number.isFinite(lineHeight) || lineHeight <= 0) throw new RangeError("Diff editor widget line height must be positive and finite");
  if (options.fontFamily !== undefined && (typeof options.fontFamily !== "string" || !options.fontFamily.trim())) throw new TypeError("Diff editor font family must be a non-empty string");
  if (options.fontSize !== undefined && (!Number.isFinite(options.fontSize) || options.fontSize <= 0)) throw new RangeError("Diff editor font size must be positive and finite");
  for (const [name, value] of [["fontLigatures", options.fontLigatures], ["showLineNumbers", options.showLineNumbers], ["showInlineChanges", options.showInlineChanges], ["loopChanges", options.loopChanges]] as const) {
    if (value !== undefined && typeof value !== "boolean") throw new TypeError(`Diff editor option '${name}' must be boolean`);
  }
  if (!Number.isSafeInteger(overscanRowCount) || overscanRowCount < 0) throw new RangeError("Diff editor widget overscan row count must be a non-negative safe integer");
  const ownerWindow = getWindow(options.container);
  if (options.container.ownerDocument.defaultView !== ownerWindow) throw new Error("Diff editor widget container must belong to its owner window");
}
