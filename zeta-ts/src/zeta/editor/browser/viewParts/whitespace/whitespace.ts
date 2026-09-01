import './whitespace.css';
import { h } from '../../../../base/browser/dom.js';
import { Position } from '../../../common/core/position.js';
import { Range } from '../../../common/core/range.js';
import { type TextModel } from '../../../common/model/textModel.js';
import { type IViewModel } from '../../../common/viewModel.js';
import { DynamicViewOverlay } from '../../view/dynamicViewOverlay.js';
import { type RenderingContext } from '../../view/renderingContext.js';
import { type ViewContext } from '../../../common/viewModel/viewContext.js';
import { type EditorVisualLineProjection } from '../../../common/viewModel/modelLineProjection.js';
import { type TextMeasurer } from '../../../common/viewModel/textMeasurer.js';
import { renderViewPartRows } from '../../view/viewLayer.js';

export type WhitespaceRenderingMode = 'none' | 'boundary' | 'selection' | 'trailing' | 'all';

/** Projects whitespace glyphs without changing the text rows used for selection geometry. */
export class WhitespaceOverlay extends DynamicViewOverlay {
	private _renderResult: string[] = [];
	constructor(
		private readonly context: ViewContext,
		private readonly model: TextModel,
		private readonly viewModel: IViewModel,
		private readonly mode: WhitespaceRenderingMode,
		private readonly ownerDocument: Document,
		private readonly readVisualProjection: () => EditorVisualLineProjection,
		private readonly readTextLeft: () => number,
		private readonly textMeasurer: TextMeasurer,
	) {
		super();
		this.context.addEventHandler(this);
	}

	public override dispose(): void {
		this.context.removeEventHandler(this);
		super.dispose();
	}

	public prepareRender(context: RenderingContext): void {
		const projection = this.readVisualProjection();
		const textLeft = this.readTextLeft();
		this._renderResult = renderViewPartRows(context, this.ownerDocument, rows => {
		for (const [visualLineIndex, row] of rows) {
			if (this.mode === 'none') {
				continue;
			}
			const visualLine = projection.lineAt(visualLineIndex);
			if (!visualLine) {
				continue;
			}
			const text = this.model.getLineContent((visualLine.logicalLineIndex) + 1).slice(visualLine.startColumn, visualLine.endColumn);
			const trailingStart = text.search(/\s*$/u);
			for (let index = 0; index < text.length; index += 1) {
				const character = text[index];
				if (character !== ' ' && character !== '\t') {
					continue;
				}
				if (this.mode === 'trailing' && index < trailingStart) {
					continue;
				}
				if (this.mode === 'boundary' && index > 0 && index < trailingStart) {
					continue;
				}
				if (this.mode === 'selection' && !this.isSelected(visualLine.logicalLineIndex, visualLine.startColumn + index)) {
					continue;
				}
				const marker = h(row.ownerDocument, 'span');
				marker.className = 'mwh stanza-editor-whitespace';
				marker.textContent = character === '\t' ? '→' : '·';
				marker.style.left = `${textLeft + this.textMeasurer.measureLineWidth(text.slice(0, index))}px`;
				row.append(marker);
			}
		}
		});
	}

	public render(startLineNumber: number, lineNumber: number): string {
		return this._renderResult[lineNumber - startLineNumber] ?? '';
	}

	private isSelected(lineIndex: number, columnIndex: number): boolean {
		const characterRange = Range.fromPositions(new Position((lineIndex) + 1, (columnIndex) + 1), new Position((lineIndex) + 1, (columnIndex + 1) + 1));
		return this.viewModel.getCursorStates().some(state => Range.areIntersecting(state.modelState.selection, characterRange));
	}
}
