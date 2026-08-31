import { h } from '../../../base/browser/dom.js';
import { ViewEventHandler } from '../../common/viewEventHandler.js';
import { type EditorOverlayContext, type EditorRenderingContext } from './renderingContext.js';

/** Base for overlays rendered from one editor view context. */
export abstract class DynamicViewOverlay extends ViewEventHandler {
	private renderResult = new Map<number, string>();

	public abstract prepareRender(context: EditorRenderingContext): void;

	public render(_startLineNumber: number, lineNumber: number): string {
		return this.renderResult.get(lineNumber) ?? '';
	}

	protected prepareRows(
		context: EditorRenderingContext,
		project: (overlay: EditorOverlayContext, rows: ReadonlyMap<number, HTMLElement>) => void,
	): void {
		const overlay = context.overlay;
		if (!overlay) {
			this.renderResult.clear();
			return;
		}
		const rows = new Map<number, HTMLElement>();
		for (let lineIndex = context.layout.renderLines.startLineIndex; lineIndex < context.layout.renderLines.endLineIndexExclusive; lineIndex += 1) {
			rows.set(lineIndex, h(overlay.ownerDocument, 'div'));
		}
		project(overlay, rows);
		this.renderResult = new Map([...rows].map(([lineIndex, row]) => [lineIndex + 1, row.innerHTML]));
	}
}
