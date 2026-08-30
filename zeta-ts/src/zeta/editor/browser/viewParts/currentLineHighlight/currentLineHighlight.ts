import './currentLineHighlight.css';
import { type CursorsController } from '../../../common/cursor/cursor.js';
import { EditorDynamicViewOverlay } from '../../view/editorDynamicViewOverlay.js';
import { type EditorRenderingContext, EditorViewContext } from '../../view/viewPart.js';
import { ViewPartRows } from '../../view/viewLayer.js';

/** Projects the active logical line independently from selection ranges. */
export class CurrentLineHighlightOverlay extends EditorDynamicViewOverlay {
	public readonly domNode: HTMLElement;
	private readonly selectionController: CursorsController | undefined;
	private readonly rows: ViewPartRows;

	constructor(context: EditorViewContext, host: HTMLElement, selectionController: CursorsController | undefined) {
		super(context);
		this.rows = this._register(new ViewPartRows(host, 'stanza-editor-current-line-highlight-layer', 'stanza-editor-current-line-highlight'));
		this.domNode = this.rows.domNode;
		this.selectionController = selectionController;
	}

	public render(context: EditorRenderingContext): void {
		const overlay = context.overlay;
		if (!overlay) {
			return;
		}
		const selections = this.selectionController?.selections.selections ?? [];
		const activeLineIndexes = new Set(selections.map(selection => selection.getPosition().lineNumber - 1));
		const selectionIsEmpty = selections.every(selection => selection.isEmpty());
		for (const [visualLineIndex, row] of this.rows.render(context)) {
			const isActive = activeLineIndexes.has(overlay.visualLineProjection.lineAt(visualLineIndex)?.logicalLineIndex ?? -1);
			const highlightsLine = selectionIsEmpty && (overlay.renderLineHighlight === 'line' || overlay.renderLineHighlight === 'all');
			const highlightsGutter = overlay.renderLineHighlight === 'gutter' || overlay.renderLineHighlight === 'all';
			row.classList.toggle('active', isActive);
			row.classList.toggle('highlight-line', highlightsLine);
			row.classList.toggle('highlight-gutter', highlightsGutter);
			row.classList.toggle('focus-only', overlay.renderLineHighlightOnlyWhenFocus);
		}
	}
}
