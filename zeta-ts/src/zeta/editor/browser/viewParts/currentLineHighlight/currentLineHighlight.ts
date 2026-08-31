import './currentLineHighlight.css';
import { type IViewModel } from '../../../common/viewModel.js';
import { DynamicViewOverlay } from '../../view/dynamicViewOverlay.js';
import { type EditorRenderingContext } from '../../view/viewPart.js';
import { type ViewContext } from '../../../common/viewModel/viewContext.js';

/** Projects the active logical line independently from selection ranges. */
export class CurrentLineHighlightOverlay extends DynamicViewOverlay {
	constructor(private readonly context: ViewContext, private readonly viewModel: IViewModel) {
		super();
		this.context.addEventHandler(this);
	}

	public override dispose(): void {
		this.context.removeEventHandler(this);
		super.dispose();
	}

	public prepareRender(context: EditorRenderingContext): void {
		const selections = this.viewModel.getCursorStates().map(state => state.modelState.selection);
		const activeLineIndexes = new Set(selections.map(selection => selection.getPosition().lineNumber - 1));
		const selectionIsEmpty = selections.every(selection => selection.isEmpty());
		this.prepareRows(context, (overlay, rows) => {
		for (const [visualLineIndex, row] of rows) {
			const isActive = activeLineIndexes.has(overlay.visualLineProjection.lineAt(visualLineIndex)?.logicalLineIndex ?? -1);
			const highlightsLine = selectionIsEmpty && (overlay.renderLineHighlight === 'line' || overlay.renderLineHighlight === 'all');
			const highlightsGutter = overlay.renderLineHighlight === 'gutter' || overlay.renderLineHighlight === 'all';
			const highlight = h(row.ownerDocument, 'div');
			highlight.className = 'current-line stanza-editor-current-line-highlight';
			highlight.classList.toggle('active', isActive);
			highlight.classList.toggle('highlight-line', highlightsLine);
			highlight.classList.toggle('highlight-gutter', highlightsGutter);
			highlight.classList.toggle('focus-only', overlay.renderLineHighlightOnlyWhenFocus);
			row.append(highlight);
		}
		});
	}
}
import { h } from '../../../../base/browser/dom.js';
