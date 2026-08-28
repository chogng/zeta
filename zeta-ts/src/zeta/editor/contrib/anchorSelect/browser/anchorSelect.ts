import './anchorSelect.css';
import { addDisposableListener, stopEvent } from '../../../../base/browser/dom.js';
import { disposableWindowTimeout } from '../../../../base/browser/scheduler.js';
import { Disposable, MutableDisposable, toDisposable, type IDisposable } from '../../../../base/common/lifecycle.js';
import { type EditorCapability, registerEditorContribution } from '../../../browser/editorExtensions.js';
import { type EditorViewport } from '../../../browser/view.js';
import { DecorationPresentation, createStanzaDecorationSource } from '../../../browser/viewparts/decorations/decorationPresentation.js';
import { type EditorSelectionController } from '../../../common/cursor/editorSelectionController.js';
import { TextSelection, TextSelectionSet } from '../../../common/core/selection.js';
import { TextRange, type TextPosition } from '../../../common/core/text.js';
import { TextDecorationCollection, type TextDecorationId } from '../../../common/model/decorationCollection.js';
import { TrackedRangeStickiness } from '../../../common/model/trackedRange.js';

const selectionAnchorDecorations: EditorCapability<TextDecorationCollection<void>> = Object.freeze({ id: 'editor.capability.selectionAnchorDecorations' });

/** Owns one editor-local selection anchor and its commands. */
export class SelectionAnchorController extends Disposable {
	private readonly chordTimeout = this._register(new MutableDisposable<IDisposable>());
	private decorationId: TextDecorationId | undefined;
	private awaitingChord = false;

	constructor(
		input: HTMLElement,
		private readonly viewport: EditorViewport,
		private readonly selections: EditorSelectionController,
		private readonly decorations: TextDecorationCollection<void>,
	) {
		super();
		try {
			if (viewport.textModel !== selections.textModel || viewport.textModel !== decorations.textModel) {
				throw new TypeError('Selection anchor dependencies must share one text model');
			}
			this._register(addDisposableListener(input, 'keydown', event => this.handleKeydown(event), true));
			this._register(toDisposable(() => this.cancelSelectionAnchor()));
		} catch (error) {
			this.dispose();
			throw error;
		}
	}

	get selectionAnchorSet(): boolean {
		return this.decorationId !== undefined;
	}

	setSelectionAnchor(): void {
		const position = this.selections.selections.primary.active;
		const [decorationId] = this.decorations.replaceAll([{
			range: TextRange.emptyAt(position),
			stickiness: TrackedRangeStickiness.NeverGrowsAtEdges,
			metadata: undefined,
		}]);
		this.decorationId = decorationId;
		this.viewport.announceAccessibilityStatus(`Anchor set at ${position.lineIndex + 1}:${position.columnIndex + 1}`);
	}

	goToSelectionAnchor(): void {
		const position = this.selectionAnchorPosition;
		if (!position) return;
		this.selections.setSelections(TextSelectionSet.single(TextSelection.collapsedAt(position)));
		this.viewport.revealPosition(position);
	}

	selectFromAnchorToCursor(): void {
		const anchor = this.selectionAnchorPosition;
		if (!anchor) return;
		const cursor = this.selections.selections.primary.active;
		this.selections.setSelections(TextSelectionSet.single(TextSelection.from(anchor, cursor)));
		this.cancelSelectionAnchor();
		this.viewport.revealPosition(cursor);
	}

	cancelSelectionAnchor(): void {
		if (this.decorationId === undefined) return;
		this.decorations.delete(this.decorationId);
		this.decorationId = undefined;
	}

	private get selectionAnchorPosition(): TextPosition | undefined {
		if (this.decorationId === undefined) return undefined;
		return this.decorations.get(this.decorationId)?.range.start;
	}

	private handleKeydown(event: KeyboardEvent): void {
		if (event.isComposing || event.getModifierState('AltGraph')) return;
		if (this.awaitingChord) {
			this.clearPendingChord();
			if (!isPrimaryChord(event)) return;
			const key = event.key.toLowerCase();
			if (key === 'b') {
				stopEvent(event, { immediate: true });
				this.setSelectionAnchor();
			} else if (key === 'k' && this.selectionAnchorSet) {
				stopEvent(event, { immediate: true });
				this.selectFromAnchorToCursor();
			}
			return;
		}
		if (!event.defaultPrevented && isPrimaryChord(event) && event.key.toLowerCase() === 'k') {
			this.awaitingChord = true;
			const targetWindow = event.view ?? (event.currentTarget instanceof HTMLElement ? event.currentTarget.ownerDocument.defaultView : undefined);
			if (targetWindow) this.chordTimeout.value = disposableWindowTimeout(targetWindow, () => this.clearPendingChord(), 5_000);
			return;
		}
		if (!event.defaultPrevented && event.key === 'Escape' && this.selectionAnchorSet) {
			stopEvent(event, { immediate: true });
			this.cancelSelectionAnchor();
		}
	}

	private clearPendingChord(): void {
		this.awaitingChord = false;
		this.chordTimeout.clear();
	}
}

registerEditorContribution({
	id: 'editor.contrib.selectionAnchorController',
	configure: context => {
		const decorations = context.register(new TextDecorationCollection<void>(context.model));
		context.provideCapability(selectionAnchorDecorations, decorations);
		context.addDecorationSource(createStanzaDecorationSource(decorations, () => DecorationPresentation.SelectionAnchor, () => 'Selection Anchor'));
	},
	install: context => {
		if (context.kind !== 'text') return;
		context.register(new SelectionAnchorController(context.view.element, context.viewport, context.selections, context.getCapability(selectionAnchorDecorations)));
	},
});

function isPrimaryChord(event: Pick<KeyboardEvent, 'ctrlKey' | 'metaKey' | 'shiftKey' | 'altKey'>): boolean {
	return (event.ctrlKey || event.metaKey) && !event.shiftKey && !event.altKey;
}
