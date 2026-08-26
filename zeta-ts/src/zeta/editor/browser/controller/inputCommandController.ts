import { stopEvent } from '../../../base/browser/dom.js';
import { DisposableOwner, toDisposable, type IDisposable } from '../../../base/common/lifecycle.js';
import { type EditorEditCommand } from '../../common/commands/editorEditCommand.js';
import { createBackspaceCommand, createDeleteForwardCommand, createDeleteToLineEndCommand, createDeleteToLineStartCommand } from '../../common/cursor/cursorDeleteOperations.js';
import { createDeleteWordBackwardCommand, createDeleteWordForwardCommand } from '../../common/cursor/cursorWordOperations.js';
import { createTypeTextCommand } from '../../common/cursor/cursorTypeOperations.js';
import { createOvertypeTextCommand } from '../../common/cursor/cursorOvertype.js';
import { type TextModelChange } from '../../common/core/text.js';
import { TextSelection, TextSelectionSet } from '../../common/core/selection.js';
import { type EditorSelectionController } from '../../common/cursor/editorSelectionController.js';
import { createLanguageCompletionIncompleteRefreshContext, createLanguageCompletionInvokeContext } from '../../common/languages/completion/languageCompletionProviders.js';
import { type EditorViewport } from '../view/editorViewport.js';
import { CompositionController } from './compositionController.js';
import { type EditContext, type EditContextTextUpdate } from './editContext/editContext.js';
import { type InputCommandTransformer, type InputCompletionRequestDelegate, type InputLanguageEditingAdapter, type InputLanguageTypeCommand } from './inputContracts.js';

export interface InputCommandControllerOptions {
	readonly completion?: InputCompletionRequestDelegate;
	readonly languageEditing?: InputLanguageEditingAdapter;
	readonly wordPattern?: () => RegExp | undefined;
}

/**
 * Converts semantic native input events into atomic common editor commands.
 *
 * This is the browser equivalent of VS Code's view/input command delegate:
 * the edit-context adapter reports what the browser attempted, while this class
 * decides which model command owns that intent.
 */
export class InputCommandController extends DisposableOwner {
	private readonly commandTransformers: InputCommandTransformer[] = [];
	private overtype = false;

	constructor(
		private readonly input: EditContext,
		private readonly viewport: EditorViewport,
		private readonly selectionController: EditorSelectionController,
		private readonly compositionController: CompositionController,
		private readonly options: InputCommandControllerOptions = {},
	) {
		super();
		this.own(input.onDidBeforeInput(event => this.handleBeforeInput(event)));
		this.own(input.onDidInput(event => {
			if (!event.isComposing || !this.compositionController.composing) this.input.clear();
		}));
		this.own(input.onDidTextUpdate(event => this.handleTextUpdate(event)));
		this.own(input.onDidKeydown(event => this.handleKeydown(event)));
	}

	get overtyping(): boolean {
		return this.overtype;
	}

	registerCommandTransformer(transformer: InputCommandTransformer): IDisposable {
		if (typeof transformer !== 'function') throw new TypeError('Text input command transformer must be a function');
		this.commandTransformers.push(transformer);
		return toDisposable(() => {
			const index = this.commandTransformers.indexOf(transformer);
			if (index >= 0) this.commandTransformers.splice(index, 1);
		});
	}

	/** Toggles this editor instance's transient overtype input mode. */
	toggleOvertype(): boolean {
		this.overtype = !this.overtype;
		this.viewport.element.classList.toggle('overtype', this.overtype);
		return this.overtype;
	}

	private handleBeforeInput(event: InputEvent): void {
		if (event.defaultPrevented || (event.isComposing && this.compositionController.composing)) return;
		const completion = this.options.completion;
		const refreshIncomplete = completion?.readIsIncomplete() ?? false;
		let insertedText: string | undefined;
		let command: EditorEditCommand | undefined;
		let languageTypeCommand: InputLanguageTypeCommand | undefined;
		switch (event.inputType) {
			case 'insertText':
			case 'insertReplacementText':
				if (!event.data) return;
				if (completion?.session?.acceptSelectedWithCommitCharacter(event.data)) {
					stopEvent(event);
					this.input.clear();
					this.revealPrimary();
					completion.requestAfterInsert(event.data, false);
					return;
				}
				languageTypeCommand = this.options.languageEditing?.createTypeCommand(this.selectionController.selections, event.data);
				insertedText = languageTypeCommand?.insertedText === false ? undefined : event.data;
				command = languageTypeCommand?.command ?? (this.overtype
					? createOvertypeTextCommand(this.viewport.textModel, this.selectionController.selections, event.data)
					: createTypeTextCommand(this.viewport.textModel, this.selectionController.selections, event.data));
				break;
			case 'insertLineBreak':
			case 'insertParagraph':
				command = this.options.languageEditing?.createEnterCommand(this.selectionController.selections) ?? createTypeTextCommand(this.viewport.textModel, this.selectionController.selections, '\n');
				break;
			case 'deleteContentBackward':
				command = this.options.languageEditing?.createBackspaceCommand(this.selectionController.selections) ?? createBackspaceCommand(
					this.viewport.textModel,
					this.selectionController.selections,
				);
				break;
			case 'deleteContentForward':
				command = createDeleteForwardCommand(
					this.viewport.textModel,
					this.selectionController.selections,
				);
				break;
			case 'deleteWordBackward':
				command = createDeleteWordBackwardCommand(
					this.viewport.textModel,
					this.selectionController.selections,
					this.currentWordPattern,
				);
				break;
			case 'deleteWordForward':
				command = createDeleteWordForwardCommand(
					this.viewport.textModel,
					this.selectionController.selections,
					this.currentWordPattern,
				);
				break;
			case 'deleteSoftLineBackward':
				command = createDeleteToLineStartCommand(
					this.viewport.textModel,
					this.selectionController.selections,
				);
				break;
			case 'deleteSoftLineForward':
				command = createDeleteToLineEndCommand(
					this.viewport.textModel,
					this.selectionController.selections,
				);
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
		stopEvent(event);
		this.input.clear();
		for (const transformer of this.commandTransformers) command = transformer(command, { inputType: event.inputType });
		const change = this.execute(command);
		if (change) languageTypeCommand?.afterExecute?.(change);
		if (insertedText !== undefined) {
			completion?.requestAfterInsert(insertedText, refreshIncomplete);
		} else if (change && refreshIncomplete) {
			completion?.requestCompletion(createLanguageCompletionIncompleteRefreshContext());
		}
	}

	/** Routes native EditContext textupdate events into the same common commands as textarea beforeinput. */
	private handleTextUpdate(update: EditContextTextUpdate): void {
		if (this.compositionController.composing) return;
		if (update.updateRangeStart === update.updateRangeEnd && update.text.length === 0) return;
		const model = this.viewport.textModel;
		const selections = this.selectionsForTextUpdate(update);
		const inputType = update.inputType ?? (update.text.length > 0 ? 'insertText' : 'deleteContentBackward');
		const completion = this.options.completion;
		const refreshIncomplete = completion?.readIsIncomplete() ?? false;
		let insertedText: string | undefined;
		let command: EditorEditCommand | undefined;
		let languageTypeCommand: InputLanguageTypeCommand | undefined;
		if (update.text.length > 0) {
			if (completion?.session?.acceptSelectedWithCommitCharacter(update.text)) {
				this.revealPrimary();
				completion.requestAfterInsert(update.text, false);
				return;
			}
			if (inputType === 'insertLineBreak' || inputType === 'insertParagraph') {
				command = this.options.languageEditing?.createEnterCommand(selections) ?? createTypeTextCommand(model, selections, update.text);
			} else {
				languageTypeCommand = this.options.languageEditing?.createTypeCommand(selections, update.text);
				insertedText = languageTypeCommand?.insertedText === false ? undefined : update.text;
				command = languageTypeCommand?.command ?? (this.overtype
					? createOvertypeTextCommand(model, selections, update.text)
					: createTypeTextCommand(model, selections, update.text));
			}
		} else if (inputType === 'deleteContentForward') {
			command = createDeleteForwardCommand(model, selections);
		} else if (inputType === 'deleteContentBackward') {
			command = this.options.languageEditing?.createBackspaceCommand(selections) ?? createBackspaceCommand(model, selections);
		} else {
			command = createTypeTextCommand(model, selections, '');
		}
		for (const transformer of this.commandTransformers) command = transformer(command, { inputType });
		const change = this.execute(command);
		if (change) languageTypeCommand?.afterExecute?.(change);
		if (insertedText !== undefined) {
			completion?.requestAfterInsert(insertedText, refreshIncomplete);
		} else if (change && refreshIncomplete) {
			completion?.requestCompletion(createLanguageCompletionIncompleteRefreshContext());
		}
	}

	private selectionsForTextUpdate(update: EditContextTextUpdate): TextSelectionSet {
		const current = this.selectionController.selections;
		const primary = current.primary;
		const primaryStart = this.viewport.textModel.offsetAt(primary.range.start);
		const primaryEnd = this.viewport.textModel.offsetAt(primary.range.end);
		if (
			primaryStart === update.previousSelectionStart &&
			primaryEnd === update.previousSelectionEnd
		) {
			return current;
		}
		return TextSelectionSet.single(TextSelection.from(
			this.viewport.textModel.positionAt(update.updateRangeStart),
			this.viewport.textModel.positionAt(update.updateRangeEnd),
		));
	}

	private handleKeydown(event: KeyboardEvent): void {
		const completion = this.options.completion;
		if (!event.defaultPrevented && !event.isComposing && !event.getModifierState('AltGraph')) {
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
		if (!event.defaultPrevented && !event.isComposing && event.key === 'Insert' && !event.shiftKey && !event.ctrlKey && !event.altKey && !event.metaKey) {
			stopEvent(event);
			this.toggleOvertype();
			return;
		}
		if (
			!event.defaultPrevented &&
			!event.isComposing &&
			event.key === ' ' &&
			event.ctrlKey &&
			!event.shiftKey &&
			!event.altKey &&
			!event.metaKey
		) {
			stopEvent(event);
			completion?.requestCompletion(createLanguageCompletionInvokeContext());
			return;
		}
		if (
			!event.defaultPrevented &&
			!event.isComposing &&
			event.altKey &&
			!event.shiftKey &&
			!event.ctrlKey &&
			!event.metaKey &&
			(event.key === 'ArrowDown' || event.key === 'ArrowUp') &&
			(event.key === 'ArrowDown'
				? completion?.session?.selectNextSnippetChoice()
				: completion?.session?.selectPreviousSnippetChoice())
		) {
			stopEvent(event);
			return;
		}
		if (
			!event.defaultPrevented &&
			!event.isComposing &&
			event.key === 'Escape' &&
			!event.shiftKey &&
			!event.ctrlKey &&
			!event.altKey &&
			!event.metaKey &&
			completion?.session?.cancelSnippetPlaceholderNavigation()
		) {
			stopEvent(event);
			return;
		}
		if (
			!event.defaultPrevented &&
			!event.isComposing &&
			event.key === 'Tab' &&
			!event.ctrlKey &&
			!event.altKey &&
			!event.metaKey &&
			(event.shiftKey
				? completion?.session?.selectPreviousSnippetPlaceholder()
				: completion?.session?.selectNextSnippetPlaceholder())
		) {
			stopEvent(event);
			return;
		}
		if (
			event.defaultPrevented ||
			event.isComposing ||
			event.key !== 'Tab' ||
			event.shiftKey ||
			event.ctrlKey ||
			event.altKey ||
			event.metaKey
		) {
			return;
		}
		if (this.selectionController.selections.selections.some(selection => !selection.collapsed)) return;
		stopEvent(event);
		this.execute(createTypeTextCommand(
			this.viewport.textModel,
			this.selectionController.selections,
			'\t',
		));
	}

	private execute(command: EditorEditCommand): TextModelChange | undefined {
		const change = this.selectionController.execute(command);
		this.revealPrimary();
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
		return this.options.wordPattern?.();
	}
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
