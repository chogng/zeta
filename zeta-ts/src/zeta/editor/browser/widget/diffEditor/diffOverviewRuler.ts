import { fragment as createFragment, h, reset } from "../../../../base/browser/dom.js";
import { FastDomNode } from "../../../../base/browser/fastDomNode.js";
import { createScrollbarAxisMetrics } from "../../../../base/browser/ui/scrollbar/scrollbarState.js";
import { LineDiffKind, type LineDiff, type LineDiffRow } from "../../../common/diff/lineDiff.js";

export const DIFF_OVERVIEW_RULER_WIDTH = 30;

export interface DiffOverviewRulerLayout {
	readonly contentHeight: number;
	readonly scrollLeft: number;
	readonly scrollTop: number;
	readonly viewportHeight: number;
	readonly viewportWidth: number;
}

/**
 * Projects aligned Diff rows into VS Code-style original and modified lanes.
 *
 * The original lane owns removed markers, the modified lane owns inserted
 * markers, and the viewport mirrors the canonical vertical scrollbar thumb.
 */
export class DiffOverviewRuler {
	readonly element: HTMLDivElement;
	private readonly root: FastDomNode<HTMLDivElement>;
	private readonly originalLane: HTMLDivElement;
	private readonly modifiedLane: HTMLDivElement;
	private readonly viewport: HTMLDivElement;
	private readonly viewportNode: FastDomNode<HTMLDivElement>;

	constructor(host: HTMLElement) {
		const ownerDocument = host.ownerDocument;
		this.element = h(ownerDocument, "div");
		this.root = new FastDomNode(this.element);
		this.originalLane = h(ownerDocument, "div");
		this.modifiedLane = h(ownerDocument, "div");
		this.viewport = h(ownerDocument, "div");
		this.viewportNode = new FastDomNode(this.viewport);
		this.root.setClassName("stanza-diff-overview");
		this.element.setAttribute("aria-hidden", "true");
		this.originalLane.className = "stanza-diff-overview-lane original";
		this.modifiedLane.className = "stanza-diff-overview-lane modified";
		this.viewportNode.setClassName("stanza-diff-overview-viewport");
		this.element.append(this.originalLane, this.modifiedLane, this.viewport);
	}

	setDiff(diff: LineDiff | undefined): void {
		const rows = diff?.rows ?? [];
		reset(this.originalLane, createMarkers(this.element.ownerDocument, rows, "original"));
		reset(this.modifiedLane, createMarkers(this.element.ownerDocument, rows, "modified"));
	}

	layout(layout: DiffOverviewRulerLayout): void {
		this.root.setLeft(layout.scrollLeft + Math.max(0, layout.viewportWidth - DIFF_OVERVIEW_RULER_WIDTH));
		this.root.setTop(layout.scrollTop);
		this.root.setHeight(layout.viewportHeight);
		const metrics = createScrollbarAxisMetrics(layout.viewportHeight, layout.contentHeight, layout.scrollTop, layout.viewportHeight, 2);
		this.viewportNode.setHeight(metrics.thumbSize);
		this.viewportNode.setTransform(`translate3d(0, ${metrics.thumbPosition}px, 0)`);
	}
}

function createMarkers(ownerDocument: Document, rows: readonly LineDiffRow[], side: "original" | "modified"): DocumentFragment {
	const fragment = createFragment(ownerDocument);
	if (rows.length === 0) return fragment;
	for (const range of changedRanges(rows, side)) {
		const marker = h(ownerDocument, "span");
		marker.className = `stanza-diff-overview-marker ${side === "original" ? "removed" : "inserted"}`;
		marker.style.top = `${range.startRow / rows.length * 100}%`;
		marker.style.height = `${(range.endRowExclusive - range.startRow) / rows.length * 100}%`;
		fragment.append(marker);
	}
	return fragment;
}

function changedRanges(rows: readonly LineDiffRow[], side: "original" | "modified"): readonly { readonly startRow: number; readonly endRowExclusive: number }[] {
	const ranges: Array<{ readonly startRow: number; readonly endRowExclusive: number }> = [];
	let startRow = -1;
	for (let rowIndex = 0; rowIndex <= rows.length; rowIndex += 1) {
		const row = rows[rowIndex];
		const changed = row !== undefined && (side === "original"
			? row.kind === LineDiffKind.Removed || row.kind === LineDiffKind.Modified
			: row.kind === LineDiffKind.Added || row.kind === LineDiffKind.Modified);
		if (changed && startRow < 0) startRow = rowIndex;
		if (!changed && startRow >= 0) {
			ranges.push(Object.freeze({ startRow, endRowExclusive: rowIndex }));
			startRow = -1;
		}
	}
	return Object.freeze(ranges);
}
