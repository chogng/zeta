import { type EditorSelectionController } from '../common/cursor/editorSelectionController.js';
import { TextPosition } from '../common/core/position.js';
import { EditorViewport } from './view.js';

/** A viewport host accepted by stable-scroll helpers. */
export interface StableEditorScrollTarget {
	readonly viewport: EditorViewport;
	readonly selections?: EditorSelectionController;
	readonly selectionController?: EditorSelectionController;
	readonly view?: {
		readonly selectionController: EditorSelectionController;
	};
}

type StableEditor = EditorViewport | StableEditorScrollTarget;

/** Preserves the first visible visual row while editor content is reprojected. */
export class StableEditorScrollState {
	private readonly initialScrollTop: number;
	private readonly initialContentHeight: number;
	private readonly visiblePosition: TextPosition | undefined;
	private readonly visiblePositionScrollDelta: number;
	private readonly cursorPosition: TextPosition | undefined;
	private readonly selectionController: EditorSelectionController | undefined;

	public static capture(
		editor: StableEditor,
		selectionController?: EditorSelectionController,
	): StableEditorScrollState {
		const viewport = resolveViewport(editor);
		const layout = viewport.currentLayout;
		const selections = resolveSelections(editor, selectionController);
		const cursorPosition = selections?.selections.primary.active;
		if (layout.scrollPosition.top === 0) {
			return new StableEditorScrollState(
				layout.scrollPosition.top,
				layout.contentSize.height,
				undefined,
				0,
				cursorPosition,
				selections,
			);
		}

		const visibleLine = viewport.getVisualLineProjection().lineAt(layout.visibleLines.startLineIndex);
		if (!visibleLine) {
			return new StableEditorScrollState(
				layout.scrollPosition.top,
				layout.contentSize.height,
				undefined,
				0,
				cursorPosition,
				selections,
			);
		}

		const visiblePosition = TextPosition.at(visibleLine.logicalLineIndex, visibleLine.startColumn);
		const visiblePositionTop = viewport.getPositionContentCoordinates(visiblePosition).top;
		return new StableEditorScrollState(
			layout.scrollPosition.top,
			layout.contentSize.height,
			visiblePosition,
			layout.scrollPosition.top - visiblePositionTop,
			cursorPosition,
			selections,
		);
	}

	private constructor(
		initialScrollTop: number,
		initialContentHeight: number,
		visiblePosition: TextPosition | undefined,
		visiblePositionScrollDelta: number,
		cursorPosition: TextPosition | undefined,
		selectionController: EditorSelectionController | undefined,
	) {
		this.initialScrollTop = initialScrollTop;
		this.initialContentHeight = initialContentHeight;
		this.visiblePosition = visiblePosition;
		this.visiblePositionScrollDelta = visiblePositionScrollDelta;
		this.cursorPosition = cursorPosition;
		this.selectionController = selectionController;
	}

	/** Restores the captured top-row offset after the viewport layout changes. */
	public restore(editor: StableEditor): void {
		const viewport = resolveViewport(editor);
		const layout = viewport.currentLayout;
		if (
			this.initialContentHeight === layout.contentSize.height &&
			this.initialScrollTop === layout.scrollPosition.top
		) {
			return;
		}
		if (!this.visiblePosition) return;

		const position = clampPosition(viewport, this.visiblePosition);
		const visiblePositionTop = viewport.getPositionContentCoordinates(position).top;
		viewport.scrollTo({
			left: layout.scrollPosition.left,
			top: visiblePositionTop + this.visiblePositionScrollDelta,
		});
	}

	/**
	 * Keeps the current cursor at the same relative vertical position after a
	 * model or visual-line projection change.
	 */
	public restoreRelativeVerticalPositionOfCursor(
		editor: StableEditor,
		selectionController?: EditorSelectionController,
	): void {
		const viewport = resolveViewport(editor);
		const layout = viewport.currentLayout;
		if (
			this.initialContentHeight === layout.contentSize.height &&
			this.initialScrollTop === layout.scrollPosition.top
		) {
			return;
		}

		const initialCursorPosition = this.cursorPosition;
		const selections = selectionController ?? this.selectionController ?? resolveSelections(editor);
		const currentCursorPosition = selections?.selections.primary.active;
		if (!initialCursorPosition || !currentCursorPosition) return;

		const initialTop = viewport.getPositionContentCoordinates(clampPosition(viewport, initialCursorPosition)).top;
		const currentTop = viewport.getPositionContentCoordinates(clampPosition(viewport, currentCursorPosition)).top;
		viewport.scrollTo({
			left: layout.scrollPosition.left,
			top: layout.scrollPosition.top + currentTop - initialTop,
		});
	}
}

/** Preserves the last visible visual row while editor content is reprojected. */
export class StableEditorBottomScrollState {
	private readonly initialScrollTop: number;
	private readonly initialContentHeight: number;
	private readonly visiblePosition: TextPosition | undefined;
	private readonly visiblePositionScrollDelta: number;

	public static capture(editor: StableEditor): StableEditorBottomScrollState {
		const viewport = resolveViewport(editor);
		const layout = viewport.currentLayout;
		const visibleLine = layout.visibleLines.endLineIndexExclusive > layout.visibleLines.startLineIndex
			? viewport.getVisualLineProjection().lineAt(layout.visibleLines.endLineIndexExclusive - 1)
			: undefined;
		if (!visibleLine) {
			return new StableEditorBottomScrollState(
				layout.scrollPosition.top,
				layout.contentSize.height,
				undefined,
				0,
			);
		}

		const visiblePosition = TextPosition.at(visibleLine.logicalLineIndex, visibleLine.startColumn);
		const coordinates = viewport.getPositionContentCoordinates(visiblePosition);
		return new StableEditorBottomScrollState(
			layout.scrollPosition.top,
			layout.contentSize.height,
			visiblePosition,
			coordinates.top + coordinates.height - layout.scrollPosition.top,
		);
	}

	private constructor(
		initialScrollTop: number,
		initialContentHeight: number,
		visiblePosition: TextPosition | undefined,
		visiblePositionScrollDelta: number,
	) {
		this.initialScrollTop = initialScrollTop;
		this.initialContentHeight = initialContentHeight;
		this.visiblePosition = visiblePosition;
		this.visiblePositionScrollDelta = visiblePositionScrollDelta;
	}

	/** Restores the captured bottom-row offset after the viewport layout changes. */
	public restore(editor: StableEditor): void {
		const viewport = resolveViewport(editor);
		const layout = viewport.currentLayout;
		if (
			this.initialContentHeight === layout.contentSize.height &&
			this.initialScrollTop === layout.scrollPosition.top
		) {
			return;
		}
		if (!this.visiblePosition) return;

		const position = clampPosition(viewport, this.visiblePosition);
		const coordinates = viewport.getPositionContentCoordinates(position);
		viewport.scrollTo({
			left: layout.scrollPosition.left,
			top: coordinates.top + coordinates.height - this.visiblePositionScrollDelta,
		});
	}
}

function resolveViewport(editor: StableEditor): EditorViewport {
	return editor instanceof EditorViewport ? editor : editor.viewport;
}

function resolveSelections(
	editor: StableEditor,
	selectionController: EditorSelectionController | undefined = undefined,
): EditorSelectionController | undefined {
	if (selectionController) return selectionController;
	if (!(editor instanceof EditorViewport)) {
		return editor.selections ?? editor.selectionController ?? editor.view?.selectionController;
	}
	return undefined;
}

function clampPosition(viewport: EditorViewport, position: TextPosition): TextPosition {
	const model = viewport.textModel;
	const lineIndex = Math.min(position.lineIndex, model.lineCount - 1);
	return TextPosition.at(lineIndex, Math.min(position.columnIndex, model.getLineLength(lineIndex)));
}
