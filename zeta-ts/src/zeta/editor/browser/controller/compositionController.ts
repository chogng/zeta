import { Emitter, type Event } from "../../../base/common/event.js";
import { IME } from "../../../base/common/ime.js";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
import { type EditorCompositionSession } from "../../common/cursor/editorComposition.js";
import { type EditorSelectionController } from "../../common/cursor/editorSelectionController.js";
import { type TextSelectionOffsets } from "../../common/commands/editorEditCommand.js";
import { normalizeTextLineEndings, type TextPosition } from "../../common/core/text.js";
import { type EditorViewport } from "../view/editorViewport.js";
import { type EditContext, type EditContextCompositionEvent } from "./editContext/editContext.js";

interface ActiveComposition {
	readonly session: EditorCompositionSession;
	text: string;
	selection: TextSelectionOffsets;
	updated: boolean;
	cancelRequested: boolean;
}

/**
 * Maps an edit-context composition stream to one protected Stanza composition session.
 */
export class CompositionController extends DisposableOwner {
	private readonly _onDidChange = this.own(new Emitter<boolean>());
	private readonly input: EditContext;
	private readonly initialReadOnly: boolean;
	private activeComposition: ActiveComposition | undefined;

	readonly onDidChange: Event<boolean> = this._onDidChange.event;

	constructor(
		input: EditContext,
		private readonly viewport: EditorViewport,
		private readonly selectionController: EditorSelectionController,
	) {
		super();
		if (viewport.textModel !== selectionController.textModel) {
			this.dispose();
			throw new TypeError(
				"Stanza composition and selection controllers must share one text model",
			);
		}
		this.input = input;
		this.initialReadOnly = input.readOnly;
		this.defer(() => {
			this.cancelComposition();
			this.input.setReadOnly(this.initialReadOnly);
			this.clearPresentation();
		});
		this.own(input.onDidCompositionStart(event => this.handleCompositionStart(event)));
		this.own(input.onDidCompositionUpdate(event => this.handleCompositionUpdate(event)));
		this.own(input.onDidCompositionEnd(event => this.handleCompositionEnd(event)));
		this.own(input.onDidKeydown(event => {
			if (event.isComposing && event.key === "Escape" && this.activeComposition) {
				this.activeComposition.cancelRequested = true;
			}
		}));
		this.own(input.onDidBlur(() => this.cancelComposition()));
		this.own(IME.onDidChange(enabled => {
			if (!enabled) this.cancelComposition();
			this.synchronizeReadOnly();
		}));
		this.own(selectionController.onDidChange(() => {
			this.finishInvalidComposition();
		}));
		this.own(viewport.textModel.onDidChange(() => {
			this.finishInvalidComposition();
		}));
		this.own(viewport.onDidChangeLayout(() => {
			if (this.activeComposition) this.positionInputAtPrimary();
		}));
		this.synchronizeReadOnly();
	}

	get composing(): boolean {
		return Boolean(this.activeComposition?.session.active);
	}

	private handleCompositionStart(event: EditContextCompositionEvent): void {
		if (event.browserEvent.defaultPrevented || this.activeComposition) return;
		if (
			!IME.enabled ||
			this.selectionController.readOnly ||
			this.selectionController.selections.selections.length !== 1
		) {
			event.browserEvent.preventDefault();
			return;
		}
		this.input.prepareComposition();
		const startPosition = this.selectionController.selections.primary.range.start;
		const session = this.selectionController.beginComposition();
		this.activeComposition = {
			session,
			text: "",
			selection: { anchorOffset: 0, activeOffset: 0 },
			updated: false,
			cancelRequested: false,
		};
		this.viewport.element.classList.add("composing");
		this._onDidChange.fire(true);
		this.viewport.revealPosition(startPosition);
		this.positionInput(startPosition);
	}

	private handleCompositionUpdate(event: EditContextCompositionEvent): void {
		if (event.browserEvent.defaultPrevented) return;
		this.updateComposition(event.text, event.selection);
	}

	private handleCompositionEnd(event: EditContextCompositionEvent): void {
		const active = this.activeComposition;
		if (!active) return;
		if (!active.session.active) {
			this.activeComposition = undefined;
			this.finishPresentation();
			return;
		}
		if (active.cancelRequested) {
			this.cancelComposition();
			return;
		}
		this.updateComposition(event.text, event.selection);
		const current = this.activeComposition;
		if (!current?.session.active) {
			this.finishPresentation();
			return;
		}
		this.activeComposition = undefined;
		current.session.commit();
		this.finishPresentation();
	}

	private updateComposition(text: string, selection: TextSelectionOffsets): void {
		const active = this.activeComposition;
		if (!active) return;
		if (!active.session.active) {
			this.activeComposition = undefined;
			this.finishPresentation();
			return;
		}
		text = normalizeTextLineEndings(text);
		if (
			active.updated &&
			active.text === text &&
			selectionsEqual(active.selection, selection)
		) {
			this.positionInputAtPrimary();
			return;
		}
		try {
			active.session.update({ text, selection });
		} catch (error) {
			if (!active.session.active) {
				this.finishInvalidComposition();
				return;
			}
			throw error;
		}
		if (this.activeComposition !== active || !active.session.active) {
			this.finishInvalidComposition();
			return;
		}
		active.text = text;
		active.selection = selection;
		active.updated = true;
		this.viewport.setCompositionRange(active.session.currentRange);
		this.viewport.revealPosition(
			this.selectionController.selections.primary.active,
		);
		this.positionInputAtPrimary();
	}

	private cancelComposition(): void {
		const active = this.activeComposition;
		if (!active) return;
		this.activeComposition = undefined;
		if (active.session.active) active.session.cancel();
		this.finishPresentation();
	}

	private finishInvalidComposition(): void {
		const active = this.activeComposition;
		if (!active || active.session.active) return;
		this.activeComposition = undefined;
		this.finishPresentation();
	}

	private positionInputAtPrimary(): void {
		this.positionInput(this.selectionController.selections.primary.active);
	}

	private positionInput(position: TextPosition): void {
		const coordinates = this.viewport.getPositionContentCoordinates(position);
		this.input.positionComposition(coordinates);
	}

	private finishPresentation(): void {
		const changed = this.clearPresentation();
		if (changed) this._onDidChange.fire(false);
	}

	private clearPresentation(): boolean {
		const changed = this.viewport.element.classList.contains("composing") ||
			this.input.element.classList.contains("ime-input");
		this.viewport.element.classList.remove("composing");
		this.input.clearComposition();
		this.viewport.setCompositionRange(undefined);
		return changed;
	}

	private synchronizeReadOnly(): void {
		this.input.setReadOnly(this.initialReadOnly || !IME.enabled);
	}
}

function selectionsEqual(left: TextSelectionOffsets, right: TextSelectionOffsets): boolean {
	return left.anchorOffset === right.anchorOffset &&
		left.activeOffset === right.activeOffset;
}
