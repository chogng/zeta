import { stopEvent } from '../../../base/browser/dom.js';
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
import { type EditContext, type EditContextTextUpdate } from '../controller/editContext/editContext.js';
import { type CompositionController } from '../controller/compositionController.js';
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
 * Routes browser input into common editor commands.
 *
 * This is the Stanza equivalent of VS Code's ViewController: EditContext
 * remains a platform adapter, while this class owns input intent, command
 * transformation, overtype, and the contribution-facing input events.
 */
export class ViewController extends DisposableOwner {
	private readonly willBeforeInputEmitter = this.own(new Emitter<InputEvent>());
	private readonly willTextUpdateEmitter = this.own(new Emitter<EditorViewTextUpdateEvent>());
	private readonly willKeydownEmitter = this.own(new Emitter<KeyboardEvent>());
	private readonly didEditEmitter = this.own(new Emitter<EditorViewDidEditEvent>());
	private readonly commandTransformers: EditorCommandTransformer[] = [];
	private readonly languageEditing: EditorLanguageEditingAdapter | undefined;
	private readonly wordPattern: (() => RegExp | undefined) | undefined;
	private readonly userInputEvents: ViewUserInputEvents;
	private overtype = false;

	readonly onWillBeforeInput: Event<InputEvent> = this.willBeforeInputEmitter.event;
	readonly onWillTextUpdate: Event<EditorViewTextUpdateEvent> = this.willTextUpdateEmitter.event;
	readonly onWillKeydown: Event<KeyboardEvent> = this.willKeydownEmitter.event;
	readonly onDidEdit: Event<EditorViewDidEditEvent> = this.didEditEmitter.event;

	constructor(
		private readonly input: EditContext,
		private readonly viewport: EditorViewport,
		private readonly selectionController: EditorSelectionController,
		private readonly compositionController: CompositionController,
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
			this.own(input.onDidBeforeInput(event => this.handleBeforeInput(event)));
			this.own(input.onDidInput(event => {
				if (!event.isComposing || !this.compositionController.composing) this.input.clear();
			}));
			this.own(input.onDidTextUpdate(event => this.handleTextUpdate(event)));
			this.own(input.onDidKeydown(event => this.handleKeydown(event)));
		} catch (error) {
			this.dispose();
			throw error;
		}
	}

	get overtyping(): boolean {
		return this.overtype;
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
		this.viewport.element.classList.toggle('overtype', this.overtype);
		return this.overtype;
	}

	private handleBeforeInput(event: InputEvent): void {
		if (event.defaultPrevented || (event.isComposing && this.compositionController.composing)) return;
		this.willBeforeInputEmitter.fire(event);
		if (event.defaultPrevented) return;

		let insertedText: string | undefined;
		let command: EditorEditCommand | undefined;
		let languageTypeCommand: EditorLanguageTypeCommand | undefined;
		switch (event.inputType) {
			case 'insertText':
			case 'insertReplacementText':
				if (!event.data) return;
				languageTypeCommand = this.languageEditing?.createTypeCommand(this.selectionController.selections, event.data);
				insertedText = languageTypeCommand?.insertedText === false ? undefined : event.data;
				command = languageTypeCommand?.command ?? (this.overtype
					? createOvertypeTextCommand(this.viewport.textModel, this.selectionController.selections, event.data)
					: createTypeTextCommand(this.viewport.textModel, this.selectionController.selections, event.data));
				break;
			case 'insertLineBreak':
			case 'insertParagraph':
				command = this.languageEditing?.createEnterCommand(this.selectionController.selections) ?? createTypeTextCommand(this.viewport.textModel, this.selectionController.selections, '\n');
				break;
			case 'deleteContentBackward':
				command = this.languageEditing?.createBackspaceCommand(this.selectionController.selections) ?? createBackspaceCommand(this.viewport.textModel, this.selectionController.selections);
				break;
			case 'deleteContentForward':
				command = createDeleteForwardCommand(this.viewport.textModel, this.selectionController.selections);
				break;
			case 'deleteWordBackward':
				command = createDeleteWordBackwardCommand(this.viewport.textModel, this.selectionController.selections, this.currentWordPattern);
				break;
			case 'deleteWordForward':
				command = createDeleteWordForwardCommand(this.viewport.textModel, this.selectionController.selections, this.currentWordPattern);
				break;
			case 'deleteSoftLineBackward':
				command = createDeleteToLineStartCommand(this.viewport.textModel, this.selectionController.selections);
				break;
			case 'deleteSoftLineForward':
				command = createDeleteToLineEndCommand(this.viewport.textModel, this.selectionController.selections);
				break;
			case 'historyUndo':
				stopEvent(event);
				this.undo();
				return;
			case 'historyRedo':
				stopEvent(event);
				this.redo();
				return;
			default:
				return;
		}
		if (!command) return;
		stopEvent(event);
		this.input.clear();
		for (const transformer of this.commandTransformers) command = transformer(command, { inputType: event.inputType });
		this.execute(command, event.inputType, insertedText, languageTypeCommand?.afterExecute);
	}

	private handleTextUpdate(update: EditContextTextUpdate): void {
		if (this.compositionController.composing) return;
		if (update.updateRangeStart === update.updateRangeEnd && update.text.length === 0) return;
		const event = makeTextUpdateEvent(update);
		this.willTextUpdateEmitter.fire(event);
		if (event.defaultPrevented) return;

		const model = this.viewport.textModel;
		const selections = this.selectionsForTextUpdate(update);
		const inputType = update.inputType ?? (update.text.length > 0 ? 'insertText' : 'deleteContentBackward');
		let insertedText: string | undefined;
		let command: EditorEditCommand;
		let languageTypeCommand: EditorLanguageTypeCommand | undefined;
		if (update.text.length > 0) {
			if (inputType === 'insertLineBreak' || inputType === 'insertParagraph') {
				command = this.languageEditing?.createEnterCommand(selections) ?? createTypeTextCommand(model, selections, update.text);
			} else {
				languageTypeCommand = this.languageEditing?.createTypeCommand(selections, update.text);
				insertedText = languageTypeCommand?.insertedText === false ? undefined : update.text;
				command = languageTypeCommand?.command ?? (this.overtype
					? createOvertypeTextCommand(model, selections, update.text)
					: createTypeTextCommand(model, selections, update.text));
			}
		} else if (inputType === 'deleteContentForward') {
			command = createDeleteForwardCommand(model, selections);
		} else if (inputType === 'deleteContentBackward') {
			command = this.languageEditing?.createBackspaceCommand(selections) ?? createBackspaceCommand(model, selections);
		} else {
			command = createTypeTextCommand(model, selections, '');
		}
		for (const transformer of this.commandTransformers) command = transformer(command, { inputType });
		this.execute(command, inputType, insertedText, languageTypeCommand?.afterExecute);
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

	private handleKeydown(event: KeyboardEvent): void {
		if (event.defaultPrevented) return;
		this.userInputEvents.emitKeyDown(event);
		this.willKeydownEmitter.fire(event);
		if (event.defaultPrevented) return;
		if (!event.isComposing && !event.getModifierState('AltGraph')) {
			if (isUndoKeybinding(event)) {
				stopEvent(event);
				this.undo();
				return;
			}
			if (isRedoKeybinding(event)) {
				stopEvent(event);
				this.redo();
				return;
			}
		}
		if (!event.isComposing && event.key === 'Insert' && !event.shiftKey && !event.ctrlKey && !event.altKey && !event.metaKey) {
			stopEvent(event);
			this.toggleOvertype();
			return;
		}
		if (
			event.isComposing ||
			event.key !== 'Tab' ||
			event.shiftKey ||
			event.ctrlKey ||
			event.altKey ||
			event.metaKey
		) return;
		if (this.selectionController.selections.selections.some(selection => !selection.collapsed)) return;
		stopEvent(event);
		// Tab is a keyboard command, not a browser text-input transaction. Keep it
		// out of post-edit consumers such as SuggestController, matching VS Code's
		// command dispatch ordering.
		this.execute(createTypeTextCommand(this.viewport.textModel, this.selectionController.selections, '\t'), 'insertText', '\t', undefined, false);
	}

	private execute(command: EditorEditCommand, inputType: string, insertedText: string | undefined, afterExecute?: (change: TextModelChange) => void, emitDidEdit = true): TextModelChange | undefined {
		const change = this.selectionController.execute(command);
		this.revealPrimary();
		if (change) {
			afterExecute?.(change);
			if (emitDidEdit) this.didEditEmitter.fire(Object.freeze({ inputType, insertedText, change }));
		}
		return change;
	}

	private undo(): void {
		this.input.clear();
		this.selectionController.undo();
		this.revealPrimary();
	}

	private redo(): void {
		this.input.clear();
		this.selectionController.redo();
		this.revealPrimary();
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

function makeTextUpdateEvent(update: EditContextTextUpdate): EditorViewTextUpdateEvent {
	let defaultPrevented = false;
	return {
		...update,
		get defaultPrevented(): boolean {
			return defaultPrevented;
		},
		preventDefault(): void {
			defaultPrevented = true;
		},
	};
}

function isUndoKeybinding(event: Pick<KeyboardEvent, 'key' | 'ctrlKey' | 'shiftKey' | 'altKey' | 'metaKey'>): boolean {
	return hasPrimaryModifier(event) && !event.shiftKey && event.key.toLowerCase() === 'z';
}

function isRedoKeybinding(event: Pick<KeyboardEvent, 'key' | 'ctrlKey' | 'shiftKey' | 'altKey' | 'metaKey'>): boolean {
	if (!hasPrimaryModifier(event)) return false;
	const key = event.key.toLowerCase();
	return (key === 'z' && event.shiftKey) || (key === 'y' && !event.shiftKey);
}

function hasPrimaryModifier(event: Pick<KeyboardEvent, 'ctrlKey' | 'altKey' | 'metaKey'>): boolean {
	return !event.altKey && event.ctrlKey !== event.metaKey;
}
