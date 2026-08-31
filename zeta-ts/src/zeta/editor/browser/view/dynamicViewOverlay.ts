import { h } from '../../../base/browser/dom.js';
import { ViewEventHandler } from '../../common/viewEventHandler.js';
import { type RenderingContext } from './renderingContext.js';

/** Base for overlays rendered from one editor view context. */
export abstract class DynamicViewOverlay extends ViewEventHandler {
	private renderResult = new Map<number, string>();

	public abstract prepareRender(context: RenderingContext): void;

	public render(_startLineNumber: number, lineNumber: number): string {
		return this.renderResult.get(lineNumber) ?? '';
	}

	protected prepareRows(
		context: RenderingContext,
		ownerDocument: Document,
		project: (rows: ReadonlyMap<number, HTMLElement>) => void,
	): void {
		const rows = new Map<number, HTMLElement>();
		for (let lineNumber = context.viewportData.startLineNumber; lineNumber <= context.viewportData.endLineNumber; lineNumber += 1) {
			rows.set(lineNumber - 1, h(ownerDocument, 'div'));
		}
		project(rows);
		this.renderResult = new Map([...rows].map(([lineIndex, row]) => [lineIndex + 1, row.innerHTML]));
	}
}
