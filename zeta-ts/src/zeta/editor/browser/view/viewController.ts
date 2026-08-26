import { Emitter, type Event } from '../../../base/common/event.js';
import { DisposableOwner, toDisposable, type IDisposable } from '../../../base/common/lifecycle.js';
import { type EditorEditCommand } from '../../common/commands/editorEditCommand.js';
import { createBackspaceCommand, createDeleteForwardCommand, createDeleteToLineEndCommand, createDeleteToLineStartCommand } from '../../common/cursor/cursorDeleteOperations.js';
import { createDeleteWordBackwardCommand, createDeleteWordForwardCommand } from '../../common/cursor/cursorWordOperations.js';
import { type EditorSelectionController } from '../../common/cursor/editorSelectionController.js';
import { createOvertypeTextCommand } from '../../common/cursor/cursorOvertype.js';
import { createTypeTextCommand } from '../../common/cursor/cursorTypeOperations.js';
import { TextSelection, TextSelectionSet } from '../../common/core/selection.js';
import { type TextModelChange } from '../../common/core/text.js';
import { type TextModel } from '../../common/model/textModel.js';
import { type EditorViewport } from '../view.js';
import { type EditContextTextUpdate } from '../controller/editContext/editContext.js';
import { ViewUserInputEvents, type EditorViewMouseEvent, type EditorViewPartialMouseEvent } from './viewUserInputEvents.js';

export interface EditorCommandContext {
	readonly inputType: string;
}

/** Extends one browser edit command before it becomes an atomic model transaction. */
export type EditorCommandTransformer = (command: EditorEditCommand, context: EditorCommandContext) => EditorEditCommand;

export interface EditorLanguageTypeCommand {
	readonly command: EditorEditCommand;
	readonly insertedText: boolean;
	afterExecute?(change: TextModelChange): void;
}

/** Optional language-aware editing seam implemented by editor contributions. */
export interface EditorLanguageEditingAdapter extends IDisposable {
	readonly textModel: TextModel;
	createTypeCommand(selections: TextSelectionSet, text: string): EditorLanguageTypeCommand | undefined;
	createEnterCommand(selections: TextSelectionSet): EditorEditCommand | undefined;
	createBackspaceCommand(selections: TextSelectionSet): EditorEditCommand | undefined;
}

/** A native text update that can be consumed by an editor contribution before model routing. */
export interface EditorViewTextUpdateEvent extends EditContextTextUpdate {
	readonly defaultPrevented: boolean;
	preventDefault(): void;
}

/** One committed browser edit reported to editor contributions. */
export interface EditorViewDidEditEvent {
	readonly inputType: string;
	readonly insertedText: string | undefined;
	readonly change: TextModelChange;
}

export interface ViewControllerOptions {
	readonly languageEditing?: EditorLanguageEditingAdapter;
	readonly wordPattern?: () => RegExp | undefined;
	readonly userInputEvents?: ViewUserInputEvents;
}

/**
 * Routes semantic editor commands into common editing operations.
 *
 * This is the Stanza equivalent of VS Code's ViewController: browser input
 * adapters normalize raw events, while this class owns command execution,
 * command transformation, overtype, and contribution-facing edit events.
 */
export class ViewController extends DisposableOwner {
	private readonly didChangeOvertypeEmitter = this.own(new Emitter<boolean>());
	private readonly didEditEmitter = this.own(new Emitter<EditorViewDidEditEvent>());
	private readonly commandTransformers: EditorCommandTransformer[] = [];
	private readonly languageEditing: EditorLanguageEditingAdapter | undefined;
	private readonly wordPattern: (() => RegExp | undefined) | undefined;
	private readonly userInputEvents: ViewUserInputEvents;
	private overtype = false;

	readonly onDidChangeOvertype: Event<boolean> = this.didChangeOvertypeEmitter.event;
	readonly onDidEdit: Event<EditorViewDidEditEvent> = this.didEditEmitter.event;

	constructor(
		private readonly viewport: EditorViewport,
		private readonly selectionController: EditorSelectionController,
		options: ViewControllerOptions = {},
	) {
		super();
		try {
			if (viewport.textModel !== selectionController.textModel) {
				throw new TypeError('Stanza view and selection controllers must share one text model');
			}
			if (options.languageEditing && options.languageEditing.textModel !== viewport.textModel) {
				throw new TypeError('Stanza view language editing must share its text model');
			}
			if (options.wordPattern !== undefined && typeof options.wordPattern !== 'function') {
				throw new TypeError('Stanza view word pattern resolver must be a function');
			}
			this.languageEditing = options.languageEditing;
			this.wordPattern = options.wordPattern;
			this.userInputEvents = options.userInputEvents ?? new ViewUserInputEvents();
		} catch (error) {
			this.dispose();
			throw error;
		}
	}

	get overtyping(): boolean {
		return this.overtype;
	}

	get hasExpandedSelections(): boolean {
		return this.selectionController.selections.selections.some(selection => !selection.collapsed);
	}

	registerCommandTransformer(transformer: EditorCommandTransformer): IDisposable {
		if (typeof transformer !== 'function') throw new TypeError('Stanza view command transformer must be a function');
		this.commandTransformers.push(transformer);
		return toDisposable(() => {
			const index = this.commandTransformers.indexOf(transformer);
			if (index >= 0) this.commandTransformers.splice(index, 1);
		});
	}

	toggleOvertype(): boolean {
		this.overtype = !this.overtype;
		this.didChangeOvertypeEmitter.fire(this.overtype);
		return this.overtype;
	}

	public type(text: string, inputType = 'insertText'): TextModelChange | undefined {
		return this.executeType(this.selectionController.selections, text, inputType);
	}

	public enter(inputType = 'insertLineBreak'): TextModelChange | undefined {
		return this.executeEnter(this.selectionController.selections, inputType);
	}

	public deleteBackward(inputType = 'deleteContentBackward'): TextModelChange | undefined {
		return this.execute(
			this.languageEditing?.createBackspaceCommand(this.selectionController.selections) ?? createBackspaceCommand(this.viewport.textModel, this.selectionController.selections),
			inputType,
		);
	}

	public deleteForward(inputType = 'deleteContentForward'): TextModelChange | undefined {
		return this.execute(createDeleteForwardCommand(this.viewport.textModel, this.selectionController.selections), inputType);
	}

	public deleteWordBackward(inputType = 'deleteWordBackward'): TextModelChange | undefined {
		return this.execute(createDeleteWordBackwardCommand(this.viewport.textModel, this.selectionController.selections, this.currentWordPattern), inputType);
	}

	public deleteWordForward(inputType = 'deleteWordForward'): TextModelChange | undefined {
		return this.execute(createDeleteWordForwardCommand(this.viewport.textModel, this.selectionController.selections, this.currentWordPattern), inputType);
	}

	public deleteSoftLineBackward(inputType = 'deleteSoftLineBackward'): TextModelChange | undefined {
		return this.execute(createDeleteToLineStartCommand(this.viewport.textModel, this.selectionController.selections), inputType);
	}

	public deleteSoftLineForward(inputType = 'deleteSoftLineForward'): TextModelChange | undefined {
		return this.execute(createDeleteToLineEndCommand(this.viewport.textModel, this.selectionController.selections), inputType);
	}

	public insertTab(): TextModelChange | undefined {
		return this.execute(createTypeTextCommand(this.viewport.textModel, this.selectionController.selections, '\t'), 'insertText', '\t', undefined, false);
	}

	public applyTextUpdate(update: EditContextTextUpdate): TextModelChange | undefined {
		const model = this.viewport.textModel;
		const selections = this.selectionsForTextUpdate(update);
		const inputType = update.inputType ?? (update.text.length > 0 ? 'insertText' : 'deleteContentBackward');
		if (update.text.length > 0) {
			if (inputType === 'insertLineBreak' || inputType === 'insertParagraph') return this.executeEnter(selections, inputType, update.text);
			return this.executeType(selections, update.text, inputType);
		}
		if (inputType === 'deleteContentForward') return this.execute(createDeleteForwardCommand(model, selections), inputType);
		if (inputType === 'deleteContentBackward') {
			return this.execute(
				this.languageEditing?.createBackspaceCommand(selections) ?? createBackspaceCommand(model, selections),
				inputType,
			);
		}
		return this.execute(createTypeTextCommand(model, selections, ''), inputType);
	}

	public undo(): void {
		this.selectionController.undo();
		this.revealPrimary();
	}

	public redo(): void {
		this.selectionController.redo();
		this.revealPrimary();
	}

	private executeType(selections: TextSelectionSet, text: string, inputType: string): TextModelChange | undefined {
		const languageTypeCommand = this.languageEditing?.createTypeCommand(selections, text);
		const insertedText = languageTypeCommand?.insertedText === false ? undefined : text;
		const command = languageTypeCommand?.command ?? (this.overtype
			? createOvertypeTextCommand(this.viewport.textModel, selections, text)
			: createTypeTextCommand(this.viewport.textModel, selections, text));
		return this.execute(command, inputType, insertedText, languageTypeCommand?.afterExecute);
	}

	private executeEnter(selections: TextSelectionSet, inputType: string, text = '\n'): TextModelChange | undefined {
		const command = this.languageEditing?.createEnterCommand(selections) ?? createTypeTextCommand(this.viewport.textModel, selections, text);
		return this.execute(command, inputType);
	}

	private selectionsForTextUpdate(update: EditContextTextUpdate): TextSelectionSet {
		const current = this.selectionController.selections;
		const primary = current.primary;
		const primaryStart = this.viewport.textModel.offsetAt(primary.range.start);
		const primaryEnd = this.viewport.textModel.offsetAt(primary.range.end);
		if (primaryStart === update.previousSelectionStart && primaryEnd === update.previousSelectionEnd) return current;
		return TextSelectionSet.single(TextSelection.from(
			this.viewport.textModel.positionAt(update.updateRangeStart),
			this.viewport.textModel.positionAt(update.updateRangeEnd),
		));
	}

	private execute(command: EditorEditCommand, inputType: string, insertedText: string | undefined = undefined, afterExecute?: (change: TextModelChange) => void, emitDidEdit = true): TextModelChange | undefined {
		const change = this.selectionController.execute(command);
		this.revealPrimary();
		if (change) {
			afterExecute?.(change);
			if (emitDidEdit) this.didEditEmitter.fire(Object.freeze({ inputType, insertedText, change }));
		}
		return change;
	}

	private revealPrimary(): void {
		this.viewport.revealPosition(this.selectionController.selections.primary.active);
	}

	private get currentWordPattern(): RegExp | undefined {
		return this.wordPattern?.();
	}

	/** Forwards view-originated input without taking ownership of its policy. */
	emitKeyDown(event: KeyboardEvent): void {
		this.userInputEvents.emitKeyDown(event);
	}

	emitKeyUp(event: KeyboardEvent): void {
		this.userInputEvents.emitKeyUp(event);
	}

	emitContextMenu(event: EditorViewMouseEvent): void {
		this.userInputEvents.emitContextMenu(event);
	}

	emitMouseMove(event: EditorViewMouseEvent): void {
		this.userInputEvents.emitMouseMove(event);
	}

	emitMouseLeave(event: EditorViewPartialMouseEvent): void {
		this.userInputEvents.emitMouseLeave(event);
	}

	emitMouseDown(event: EditorViewMouseEvent): void {
		this.userInputEvents.emitMouseDown(event);
	}

	emitMouseUp(event: EditorViewMouseEvent): void {
		this.userInputEvents.emitMouseUp(event);
	}

	emitMouseDrag(event: EditorViewMouseEvent): void {
		this.userInputEvents.emitMouseDrag(event);
	}

	emitMouseDrop(event: EditorViewPartialMouseEvent): void {
		this.userInputEvents.emitMouseDrop(event);
	}

	emitMouseDropCanceled(): void {
		this.userInputEvents.emitMouseDropCanceled();
	}

	emitMouseWheel(event: WheelEvent): void {
		this.userInputEvents.emitMouseWheel(event);
	}
}
