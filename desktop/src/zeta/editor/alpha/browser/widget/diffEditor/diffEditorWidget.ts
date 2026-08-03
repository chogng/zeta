import "./diffEditorWidget.css";
import { addDisposableListener, reset, stopEvent } from "../../../../../base/browser/dom.js";
import { getClientArea, type IDimension } from "../../../../../base/browser/geometry.js";
import { getWindow } from "../../../../../base/browser/window.js";
import { DisposableOwner } from "../../../../../base/common/lifecycle.js";
import { DiffModel } from "../../../common/models/diff/diffModel.js";
import { LineDiffKind, type DiffRange, type LineDiff, type LineDiffRow } from "../../../common/models/diff/lineDiff.js";

const DEFAULT_LINE_HEIGHT = 20;
const DEFAULT_OVERSCAN_ROW_COUNT = 8;

export interface DiffEditorWidgetOptions {
  readonly container: HTMLElement;
  readonly model: DiffModel;
  readonly lineHeight?: number;
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

  constructor(options: DiffEditorWidgetOptions) {
    super();
    validateOptions(options);
    const ownerDocument = options.container.ownerDocument;
    this.model = options.model;
    this.lineHeight = options.lineHeight ?? DEFAULT_LINE_HEIGHT;
    this.overscanRowCount = options.overscanRowCount ?? DEFAULT_OVERSCAN_ROW_COUNT;
    this.currentDiff = this.model.diff;
    this.element = ownerDocument.createElement("div");
    this.contentElement = ownerDocument.createElement("div");
    this.rowsElement = ownerDocument.createElement("div");
    this.accessibilityStatusElement = ownerDocument.createElement("div");
    this.element.className = "zeta-alpha-diff-editor";
    this.element.tabIndex = 0;
    this.element.setAttribute("role", "region");
    this.element.setAttribute("aria-label", `Side-by-side diff editor. Original: ${options.originalAriaLabel ?? "Original"}. Modified: ${options.modifiedAriaLabel ?? "Modified"}.`);
    this.contentElement.className = "zeta-alpha-diff-editor-content";
    this.rowsElement.className = "zeta-alpha-diff-editor-rows";
    this.accessibilityStatusElement.className = "zeta-alpha-diff-editor-accessibility-status";
    this.accessibilityStatusElement.setAttribute("aria-live", "polite");
    this.accessibilityStatusElement.setAttribute("aria-atomic", "true");
    this.contentElement.append(this.rowsElement);
    this.element.append(this.contentElement, this.accessibilityStatusElement);
    options.container.append(this.element);
    this.defer(() => this.element.remove());
    this.own(addDisposableListener(this.element, "scroll", () => this.project()));
    this.own(addDisposableListener(this.element, "keydown", event => this.handleKeydown(event)));
    this.own(this.model.onDidChange(() => this.refresh()));
    const ResizeObserverConstructor = ownerDocument.defaultView?.ResizeObserver;
    if (ResizeObserverConstructor) {
      const observer = new ResizeObserverConstructor(([entry]) => {
        if (entry) this.layout({ width: entry.contentRect.width, height: entry.contentRect.height });
      });
      observer.observe(this.element);
      this.defer(() => observer.disconnect());
    }
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
      throw new RangeError("Alpha diff line index must be a non-negative safe integer");
    }
    const diff = this.currentDiff;
    if (!diff) throw new Error("Diff results are not ready");
    const rowIndex = diff.rows.findIndex(row => row[side] === lineIndex);
    if (rowIndex < 0) throw new RangeError("Alpha diff line index is outside its source model");
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
      : (currentIndex + delta + changedRows.length) % changedRows.length;
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
    const fragment = this.element.ownerDocument.createDocumentFragment();
    for (let rowIndex = startRow; rowIndex < endRow; rowIndex += 1) {
      const row = rows[rowIndex]!;
      fragment.append(createDiffRow(this.element.ownerDocument, row, this.model.original, this.model.modified, this.lineHeight, rowIndex === this.activeChangeRow));
    }
    this.rowsElement.style.transform = `translate3d(0, ${startRow * this.lineHeight}px, 0)`;
    reset(this.rowsElement, fragment);
    this.renderedStartRow = startRow;
    this.renderedEndRow = endRow;
  }
}

function createDiffRow(ownerDocument: Document, row: LineDiffRow, original: DiffModel["original"], modified: DiffModel["modified"], lineHeight: number, active: boolean): HTMLDivElement {
  const element = ownerDocument.createElement("div");
  element.className = `zeta-alpha-diff-row ${row.kind}`;
  element.classList.toggle("active", active);
  element.style.height = `${lineHeight}px`;
  element.style.lineHeight = `${lineHeight}px`;
  element.append(
    createDiffCell(ownerDocument, "original", row.kind, row.originalLineIndex, row.originalLineIndex === undefined ? undefined : original.getLineContent(row.originalLineIndex), row.originalChanges),
    createDiffCell(ownerDocument, "modified", row.kind, row.modifiedLineIndex, row.modifiedLineIndex === undefined ? undefined : modified.getLineContent(row.modifiedLineIndex), row.modifiedChanges),
  );
  return element;
}

function diffRowLocation(row: LineDiffRow): string {
  const original = row.originalLineIndex === undefined ? "no original line" : `original line ${row.originalLineIndex + 1}`;
  const modified = row.modifiedLineIndex === undefined ? "no modified line" : `modified line ${row.modifiedLineIndex + 1}`;
  return `${original}, ${modified}`;
}

function createDiffCell(ownerDocument: Document, side: "original" | "modified", kind: LineDiffKind, lineIndex: number | undefined, text: string | undefined, changes: readonly DiffRange[]): HTMLDivElement {
  const cell = ownerDocument.createElement("div");
  cell.className = `zeta-alpha-diff-cell ${side}`;
  const number = ownerDocument.createElement("span");
  number.className = "zeta-alpha-diff-line-number";
  number.textContent = lineIndex === undefined ? "" : String(lineIndex + 1);
  const content = ownerDocument.createElement("span");
  content.className = "zeta-alpha-diff-line-content";
  if (text === undefined) {
    cell.classList.add("missing");
  } else {
    projectDiffText(ownerDocument, content, text, changes, side === "original" ? LineDiffKind.Removed : LineDiffKind.Added);
    if (kind === LineDiffKind.Modified) cell.classList.add(side === "original" ? "removed" : "added");
    else if (kind === LineDiffKind.Removed && side === "original") cell.classList.add("removed");
    else if (kind === LineDiffKind.Added && side === "modified") cell.classList.add("added");
  }
  cell.append(number, content);
  return cell;
}

function projectDiffText(ownerDocument: Document, target: HTMLElement, text: string, changes: readonly DiffRange[], changedKind: LineDiffKind): void {
  const fragment = ownerDocument.createDocumentFragment();
  let previousEnd = 0;
  for (const change of changes) {
    fragment.append(text.slice(previousEnd, change.startColumn));
    const changed = ownerDocument.createElement("span");
    changed.className = `zeta-alpha-diff-inline ${changedKind}`;
    changed.textContent = text.slice(change.startColumn, change.endColumn);
    fragment.append(changed);
    previousEnd = change.endColumn;
  }
  fragment.append(text.slice(previousEnd));
  reset(target, fragment);
}

function validateOptions(options: DiffEditorWidgetOptions): void {
  if (!options || typeof options !== "object" || !isHtmlElement(options.container)) {
    throw new TypeError("Diff editor widget requires a browser container");
  }
  if (!options.model || typeof options.model !== "object") {
    throw new TypeError("Diff editor widget requires a diff model");
  }
  const lineHeight = options.lineHeight ?? DEFAULT_LINE_HEIGHT;
  const overscanRowCount = options.overscanRowCount ?? DEFAULT_OVERSCAN_ROW_COUNT;
  if (!Number.isFinite(lineHeight) || lineHeight <= 0) throw new RangeError("Diff editor widget line height must be positive and finite");
  if (!Number.isSafeInteger(overscanRowCount) || overscanRowCount < 0) throw new RangeError("Diff editor widget overscan row count must be a non-negative safe integer");
  const ownerWindow = getWindow(options.container);
  if (options.container.ownerDocument.defaultView !== ownerWindow) throw new Error("Diff editor widget container must belong to its owner window");
}

function isHtmlElement(value: unknown): value is HTMLElement {
  if (!value || typeof value !== "object" || !("ownerDocument" in value)) return false;
  const ownerDocument = (value as Node).ownerDocument;
  const HTMLElementConstructor = ownerDocument?.defaultView?.HTMLElement;
  return HTMLElementConstructor !== undefined && value instanceof HTMLElementConstructor;
}
