import './currentLineHighlight.css';
import { type IViewModel } from '../../../common/viewModel.js';
import { type EditorVisualLineProjection } from '../../../common/viewModel/modelLineProjection.js';
import { DynamicViewOverlay } from '../../view/dynamicViewOverlay.js';
import { type RenderingContext } from '../../view/renderingContext.js';
import { type ViewContext } from '../../../common/viewModel/viewContext.js';

/** Projects the active logical line independently from selection ranges. */
export class CurrentLineHighlightOverlay extends DynamicViewOverlay {
	constructor(
		private readonly context: ViewContext,
		private readonly viewModel: IViewModel,
		private readonly ownerDocument: Document,
		private readonly readVisualProjection: () => EditorVisualLineProjection,
		private readonly renderLineHighlight: 'none' | 'gutter' | 'line' | 'all',
		private readonly renderLineHighlightOnlyWhenFocus: boolean,
	) {
		super();
		this.context.addEventHandler(this);
	}

	public override dispose(): void {
		this.context.removeEventHandler(this);
		super.dispose();
	}

	public prepareRender(context: RenderingContext): void {
		const selections = this.viewModel.getCursorStates().map(state => state.modelState.selection);
		const activeLineIndexes = new Set(selections.map(selection => selection.getPosition().lineNumber - 1));
		const selectionIsEmpty = selections.every(selection => selection.isEmpty());
		const projection = this.readVisualProjection();
		this.prepareRows(context, this.ownerDocument, rows => {
		for (const [visualLineIndex, row] of rows) {
			const isActive = activeLineIndexes.has(projection.lineAt(visualLineIndex)?.logicalLineIndex ?? -1);
			const highlightsLine = selectionIsEmpty && (this.renderLineHighlight === 'line' || this.renderLineHighlight === 'all');
			const highlightsGutter = this.renderLineHighlight === 'gutter' || this.renderLineHighlight === 'all';
			const highlight = h(row.ownerDocument, 'div');
			highlight.className = 'current-line stanza-editor-current-line-highlight';
			highlight.classList.toggle('active', isActive);
			highlight.classList.toggle('highlight-line', highlightsLine);
			highlight.classList.toggle('highlight-gutter', highlightsGutter);
			highlight.classList.toggle('focus-only', this.renderLineHighlightOnlyWhenFocus);
			row.append(highlight);
		}
		});
	}
}
import { h } from '../../../../base/browser/dom.js';
