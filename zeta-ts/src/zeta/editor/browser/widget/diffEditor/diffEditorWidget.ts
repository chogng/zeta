import "./diffEditorWidget.css";
import { addDisposableListener, fragment as createFragment, h, isHTMLElement, reset, stopEvent } from "../../../../base/browser/dom.js";
import { FastDomNode } from "../../../../base/browser/fastDomNode.js";
import { getClientArea, type IDimension } from "../../../../base/browser/geometry.js";
import { observeResize } from "../../../../base/browser/observer.js";
import { getWindow } from "../../../../base/browser/window.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { rot } from "../../../../base/common/numbers.js";
import { DiffModel } from "../../../common/diff/diffModel.js";
import { LineDiffKind, type LineDiff, type LineDiffRow } from "../../../common/diff/lineDiff.js";
import { DiffOverviewRuler } from "./diffOverviewRuler.js";
import { createDiffEditorRow } from "./diffEditorRows.js";

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
	private readonly contentNode: FastDomNode<HTMLDivElement>;
	private readonly rowsElement: HTMLDivElement;
	private readonly rowsNode: FastDomNode<HTMLDivElement>;
	private readonly overviewRuler: DiffOverviewRuler;
	private readonly accessibilityStatusElement: HTMLDivElement;
	private readonly model: DiffModel;
	private readonly lineHeight: number;
	private readonly overscanRowCount: number;
	private currentDiff: LineDiff | undefined;
	private renderedStartRow = -1;
	private renderedEndRow = -1;
	private viewportWidth = 0;
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
		this.contentNode = new FastDomNode(this.contentElement);
		this.rowsElement = h(ownerDocument, "div");
		this.rowsNode = new FastDomNode(this.rowsElement);
		this.overviewRuler = new DiffOverviewRuler(this.element);
		this.accessibilityStatusElement = h(ownerDocument, "div");
		this.element.className = "stanza-diff-editor";
		this.element.classList.toggle("hide-line-numbers", options.showLineNumbers === false);
		if (options.fontFamily) this.element.style.fontFamily = options.fontFamily;
		if (options.fontSize !== undefined) this.element.style.fontSize = `${options.fontSize}px`;
		this.element.style.fontVariantLigatures = options.fontLigatures ? "normal" : "none";
		this.element.tabIndex = 0;
		this.element.setAttribute("role", "region");
		this.element.setAttribute("aria-label", `Side-by-side diff editor. Original: ${options.originalAriaLabel ?? "Original"}. Modified: ${options.modifiedAriaLabel ?? "Modified"}.`);
		this.contentNode.setClassName("stanza-diff-editor-content");
		this.rowsNode.setClassName("stanza-diff-editor-rows");
		this.accessibilityStatusElement.className = "stanza-diff-editor-accessibility-status";
		this.accessibilityStatusElement.setAttribute("aria-live", "polite");
		this.accessibilityStatusElement.setAttribute("aria-atomic", "true");
		this.contentElement.append(this.rowsElement);
		this.element.append(this.contentElement, this.overviewRuler.element, this.accessibilityStatusElement);
		options.container.append(this.element);
		this.defer(() => this.element.remove());
		this.own(addDisposableListener(this.element, "scroll", () => this.project()));
		this.own(addDisposableListener(this.element, "keydown", event => this.handleKeydown(event)));
		this.own(this.model.onDidChange(() => this.refresh()));
		this.own(observeResize(this.element, ([entry]) => {
			if (entry) this.layout({ width: entry.contentRect.width, height: entry.contentRect.height });
		}));
		this.overviewRuler.setDiff(this.currentDiff);
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
		this.viewportWidth = size.width;
		this.viewportHeight = size.height;
		this.project(true);
	}

	private refresh(): void {
		this.currentDiff = this.model.diff;
		this.overviewRuler.setDiff(this.currentDiff);
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
			throw new RangeError("Stanza diff line index must be a non-negative safe integer");
		}
		const diff = this.currentDiff;
		if (!diff) throw new Error("Diff results are not ready");
		const rowIndex = diff.rows.findIndex(row => row[side] === lineIndex);
		if (rowIndex < 0) throw new RangeError("Stanza diff line index is outside its source model");
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
				? rot(currentIndex + delta, changedRows.length)
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
		const contentHeight = rows.length * this.lineHeight;
		this.contentNode.setHeight(contentHeight);
		this.overviewRuler.layout({ contentHeight, scrollLeft: this.element.scrollLeft, scrollTop: this.element.scrollTop, viewportHeight: this.viewportHeight, viewportWidth: this.viewportWidth });
		const visibleRowCount = Math.ceil(this.viewportHeight / this.lineHeight);
		const firstVisibleRow = Math.floor(this.element.scrollTop / this.lineHeight);
		const startRow = Math.max(0, firstVisibleRow - this.overscanRowCount);
		const endRow = Math.min(rows.length, firstVisibleRow + visibleRowCount + this.overscanRowCount);
		if (!force && startRow === this.renderedStartRow && endRow === this.renderedEndRow) return;
		const fragment = createFragment(this.element.ownerDocument);
		for (let rowIndex = startRow; rowIndex < endRow; rowIndex += 1) {
			const row = rows[rowIndex]!;
			fragment.append(createDiffEditorRow(this.element.ownerDocument, row, this.model, this.lineHeight, rowIndex === this.activeChangeRow, this.showInlineChanges));
		}
		this.rowsNode.setTransform(`translate3d(0, ${startRow * this.lineHeight}px, 0)`);
		reset(this.rowsElement, fragment);
		this.renderedStartRow = startRow;
		this.renderedEndRow = endRow;
	}
}

function diffRowLocation(row: LineDiffRow): string {
	const original = row.originalLineIndex === undefined ? "no original line" : `original line ${row.originalLineIndex + 1}`;
	const modified = row.modifiedLineIndex === undefined ? "no modified line" : `modified line ${row.modifiedLineIndex + 1}`;
	return `${original}, ${modified}`;
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
