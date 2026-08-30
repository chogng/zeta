import { type CursorsController } from '../common/cursor/cursor.js';
import { Position } from '../common/core/position.js';
import { type View } from './view.js';

/** Preserves the first visible visual row while editor content is reprojected. */
export class StableEditorScrollState {
	private readonly initialScrollTop: number;
	private readonly initialContentHeight: number;
	private readonly visiblePosition: Position | undefined;
	private readonly visiblePositionScrollDelta: number;
	private readonly cursorPosition: Position | undefined;
	private readonly selectionController: CursorsController | undefined;

	public static capture(
		viewport: View,
		selectionController?: CursorsController,
	): StableEditorScrollState {
		const layout = viewport.currentLayout;
		const cursorPosition = selectionController?.selections.primary.getPosition();
		if (layout.scrollPosition.top === 0) {
			return new StableEditorScrollState(
				layout.scrollPosition.top,
				layout.contentSize.height,
				undefined,
				0,
				cursorPosition,
				selectionController,
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
				selectionController,
			);
		}

		const visiblePosition = new Position((visibleLine.logicalLineIndex) + 1, (visibleLine.startColumn) + 1);
		const visiblePositionTop = viewport.getPositionContentCoordinates(visiblePosition).top;
		return new StableEditorScrollState(
			layout.scrollPosition.top,
			layout.contentSize.height,
			visiblePosition,
			layout.scrollPosition.top - visiblePositionTop,
			cursorPosition,
			selectionController,
		);
	}

	private constructor(
		initialScrollTop: number,
		initialContentHeight: number,
		visiblePosition: Position | undefined,
		visiblePositionScrollDelta: number,
		cursorPosition: Position | undefined,
		selectionController: CursorsController | undefined,
	) {
		this.initialScrollTop = initialScrollTop;
		this.initialContentHeight = initialContentHeight;
		this.visiblePosition = visiblePosition;
		this.visiblePositionScrollDelta = visiblePositionScrollDelta;
		this.cursorPosition = cursorPosition;
		this.selectionController = selectionController;
	}

	/** Restores the captured top-row offset after the viewport layout changes. */
	public restore(viewport: View): void {
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
		viewport: View,
		selectionController?: CursorsController,
	): void {
		const layout = viewport.currentLayout;
		if (
			this.initialContentHeight === layout.contentSize.height &&
			this.initialScrollTop === layout.scrollPosition.top
		) {
			return;
		}

		const initialCursorPosition = this.cursorPosition;
		const selections = selectionController ?? this.selectionController;
		const currentCursorPosition = selections?.selections.primary.getPosition();
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
	private readonly visiblePosition: Position | undefined;
	private readonly visiblePositionScrollDelta: number;

	public static capture(viewport: View): StableEditorBottomScrollState {
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

		const visiblePosition = new Position((visibleLine.logicalLineIndex) + 1, (visibleLine.startColumn) + 1);
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
		visiblePosition: Position | undefined,
		visiblePositionScrollDelta: number,
	) {
		this.initialScrollTop = initialScrollTop;
		this.initialContentHeight = initialContentHeight;
		this.visiblePosition = visiblePosition;
		this.visiblePositionScrollDelta = visiblePositionScrollDelta;
	}

	/** Restores the captured bottom-row offset after the viewport layout changes. */
	public restore(viewport: View): void {
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

function clampPosition(viewport: View, position: Position): Position {
	const model = viewport.textModel;
	const lineNumber = Math.min(Math.max(position.lineNumber, 1), model.lineCount);
	return new Position(lineNumber, Math.min(Math.max(position.column, 1), model.getLineLength(lineNumber) + 1));
}
