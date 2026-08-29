import './whitespace.css';
import { h, reset } from '../../../../base/browser/dom.js';
import { type CursorsController } from '../../../common/cursor/cursor.js';
import { Position } from '../../../common/core/position.js';
import { Range } from '../../../common/core/range.js';
import { type TextModel } from '../../../common/model/textModel.js';
import { DynamicViewOverlay } from '../../view/dynamicViewOverlay.js';
import { type EditorRenderingContext, EditorViewContext } from '../../view/viewPart.js';
import { ViewPartRows } from '../../view/viewLayer.js';

export type WhitespaceRenderingMode = 'none' | 'boundary' | 'selection' | 'trailing' | 'all';

/** Projects whitespace glyphs without changing the text rows used for selection geometry. */
export class WhitespaceOverlay extends DynamicViewOverlay {
	public readonly domNode: HTMLElement;
	private readonly rows: ViewPartRows;

	constructor(
		context: EditorViewContext,
		host: HTMLElement,
		private readonly model: TextModel,
		private readonly selectionController: CursorsController | undefined,
		private readonly mode: WhitespaceRenderingMode,
	) {
		super(context);
		this.rows = this._register(new ViewPartRows(host, 'stanza-editor-whitespace-layer', 'stanza-editor-whitespace-row'));
		this.domNode = this.rows.domNode;
	}

	public render(context: EditorRenderingContext): void {
		const overlay = context.overlay;
		if (!overlay) {
			return;
		}
		for (const [visualLineIndex, row] of this.rows.render(context)) {
			reset(row);
			if (this.mode === 'none') {
				continue;
			}
			const visualLine = overlay.visualLineProjection.lineAt(visualLineIndex);
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
				marker.className = 'stanza-editor-whitespace';
				marker.textContent = character === '\t' ? '→' : '·';
				marker.style.left = `${overlay.textLeft + overlay.textMeasurer.measureLineWidth(text.slice(0, index))}px`;
				row.append(marker);
			}
		}
	}

	private isSelected(lineIndex: number, columnIndex: number): boolean {
		if (!this.selectionController) return false;
		const characterRange = Range.fromPositions(new Position((lineIndex) + 1, (columnIndex) + 1), new Position((lineIndex) + 1, (columnIndex + 1) + 1));
		return this.selectionController.selections.selections.some(selection => Range.areIntersecting(selection, characterRange));
	}
}
