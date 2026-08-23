import { addDisposableListener } from "../../../base/browser/dom.js";
import { FastDomNode } from "../../../base/browser/fastDomNode.js";
import { Emitter, type Event } from "../../../base/common/event.js";
import { IME } from "../../../base/common/ime.js";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
import { type EditorCompositionSession } from "../../common/cursor/editorComposition.js";
import { type EditorSelectionController } from "../../common/cursor/editorSelectionController.js";
import { type TextSelectionOffsets } from "../../common/commands/editorEditCommand.js";
import { normalizeTextLineEndings, type TextPosition } from "../../common/core/text.js";
import { type EditorViewport } from "../view/editorViewport.js";

interface ActiveComposition {
	readonly session: EditorCompositionSession;
	text: string;
	selection: TextSelectionOffsets;
	updated: boolean;
	cancelRequested: boolean;
}

/**
 * Maps textarea composition events to one protected Aster composition session.
 */
export class CompositionController extends DisposableOwner {
	private readonly _onDidChange = this.own(new Emitter<boolean>());
	private readonly inputNode: FastDomNode<HTMLTextAreaElement>;
	private readonly initialReadOnly: boolean;
	private activeComposition: ActiveComposition | undefined;

	readonly onDidChange: Event<boolean> = this._onDidChange.event;

	constructor(
		inputNode: FastDomNode<HTMLTextAreaElement>,
		private readonly viewport: EditorViewport,
		private readonly selectionController: EditorSelectionController,
	) {
		super();
		if (viewport.textModel !== selectionController.textModel) {
			this.dispose();
			throw new TypeError(
				"Aster composition and selection controllers must share one text model",
			);
		}
		this.inputNode = inputNode;
		const element = inputNode.domNode;
		this.initialReadOnly = element.readOnly;
		this.defer(() => {
			this.cancelComposition();
			this.inputNode.domNode.readOnly = this.initialReadOnly;
			this.clearPresentation();
		});
		this.own(addDisposableListener<CompositionEvent>(
			element,
			"compositionstart",
			event => this.handleCompositionStart(event),
		));
		this.own(addDisposableListener<CompositionEvent>(
			element,
			"compositionupdate",
			event => this.handleCompositionUpdate(event),
		));
		this.own(addDisposableListener<CompositionEvent>(
			element,
			"compositionend",
			event => this.handleCompositionEnd(event),
		));
		this.own(addDisposableListener(element, "keydown", event => {
			if (event.isComposing && event.key === "Escape" && this.activeComposition) {
				this.activeComposition.cancelRequested = true;
			}
		}));
		this.own(addDisposableListener(element, "blur", () => {
			this.cancelComposition();
		}));
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

	private handleCompositionStart(event: CompositionEvent): void {
		if (event.defaultPrevented || this.activeComposition) return;
		if (
			!IME.enabled ||
			this.selectionController.readOnly ||
			this.selectionController.selections.selections.length !== 1
		) {
			event.preventDefault();
			return;
		}
		this.inputNode.domNode.value = "";
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
		this.inputNode.toggleClassName("ime-input", true);
		this._onDidChange.fire(true);
		this.viewport.revealPosition(startPosition);
		this.positionInput(startPosition);
	}

	private handleCompositionUpdate(event: CompositionEvent): void {
		if (event.defaultPrevented) return;
		this.updateComposition(event.data);
	}

	private handleCompositionEnd(event: CompositionEvent): void {
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
		this.updateComposition(event.data);
		const current = this.activeComposition;
		if (!current?.session.active) {
			this.finishPresentation();
			return;
		}
		this.activeComposition = undefined;
		current.session.commit();
		this.finishPresentation();
	}

	private updateComposition(rawText: string): void {
		const active = this.activeComposition;
		if (!active) return;
		if (!active.session.active) {
			this.activeComposition = undefined;
			this.finishPresentation();
			return;
		}
		const text = normalizeTextLineEndings(rawText);
		const selection = readCompositionSelection(this.inputNode.domNode, rawText, text);
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
		this.inputNode.setLeft(coordinates.left);
		this.inputNode.setTop(coordinates.top);
		this.inputNode.setHeight(coordinates.height);
	}

	private finishPresentation(): void {
		this.inputNode.domNode.value = "";
		const changed = this.clearPresentation();
		if (changed) this._onDidChange.fire(false);
	}

	private clearPresentation(): boolean {
		const changed = this.viewport.element.classList.contains("composing") ||
			this.inputNode.domNode.classList.contains("ime-input");
		this.viewport.element.classList.remove("composing");
		this.inputNode.toggleClassName("ime-input", false);
		this.inputNode.setLeft("");
		this.inputNode.setTop("");
		this.inputNode.setHeight("");
		this.viewport.setCompositionRange(undefined);
		return changed;
	}

	private synchronizeReadOnly(): void {
		this.inputNode.domNode.readOnly = this.initialReadOnly || !IME.enabled;
	}
}

function readCompositionSelection(element: HTMLTextAreaElement, rawText: string, normalizedText: string): TextSelectionOffsets {
	if (element.value !== rawText) {
		return {
			anchorOffset: normalizedText.length,
			activeOffset: normalizedText.length,
		};
	}
	const start = normalizedOffset(rawText, element.selectionStart);
	const end = normalizedOffset(rawText, element.selectionEnd);
	return element.selectionDirection === "backward"
		? { anchorOffset: end, activeOffset: start }
		: { anchorOffset: start, activeOffset: end };
}

function normalizedOffset(text: string, offset: number): number {
	return normalizeTextLineEndings(text.slice(0, offset)).length;
}

function selectionsEqual(left: TextSelectionOffsets, right: TextSelectionOffsets): boolean {
	return left.anchorOffset === right.anchorOffset &&
		left.activeOffset === right.activeOffset;
}
