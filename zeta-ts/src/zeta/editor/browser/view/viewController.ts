import { type IKeyboardEvent } from '../../../base/browser/keyboardEvent.js';
import { Emitter, type Event } from '../../../base/common/event.js';
import { Disposable, toDisposable, type IDisposable } from '../../../base/common/lifecycle.js';
import { operatingSystem, OperatingSystem } from '../../../base/common/platform.js';
import { type EditorEditCommand } from '../../common/commands/editorEditCommand.js';
import { EditorLineWrapping } from '../../common/config/editorOptions.js';
import { CursorNavigation, EditorCursorNavigationCommand, EditorCursorNavigationMode } from '../../common/cursor/cursorNavigation.js';
import { SelectionSetDeleteOperations } from '../../common/cursor/selectionSetDeleteOperations.js';
import { LanguageAutoClosingTracker } from '../../common/cursor/languageAutoClosingTracker.js';
import { createLanguageEnterCommand } from '../../common/cursor/languageEnter.js';
import { createLanguagePairBackspaceCommand, createLanguagePairTypeCommand } from '../../common/cursor/languagePairEditing.js';
import { SelectionSetWordOperations } from '../../common/cursor/selectionSetWordOperations.js';
import { type CursorsController } from '../../common/cursor/cursor.js';
import { AutoClosingOvertypeOperation } from '../../common/cursor/cursorTypeEditOperations.js';
import { TypeOperations } from '../../common/cursor/cursorTypeOperations.js';
import { Selection } from '../../common/core/selection.js';
import { type Position } from '../../common/core/position.js';
import { SelectionSet } from '../../common/cursor/selectionSet.js';
import { type TextModelChange } from '../../common/core/textChange.js';
import { resolveEditorIndentationOptions, type EditorIndentationOptions } from '../../common/core/misc/indentation.js';
import { type ILanguageConfigurationService } from '../../common/languages/languageConfigurationRegistry.js';
import { type LanguageLexicalContextSource, LanguageLexicalContextIndex } from '../../common/languages/languageLexicalContext.js';
import { assertLanguageId } from '../../common/languages/languageId.js';
import { type TextModel } from '../../common/model/textModel.js';
import { navigateStanzaVisualCursors } from '../../common/viewModel/visualCursorNavigation.js';
import { type View } from '../view.js';
import { type EditContextTextUpdate } from '../controller/editContext/editContext.js';
import { EditorViewUserInputEvents, type EditorViewMouseEvent, type EditorViewPartialMouseEvent } from './viewUserInputEvents.js';

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
	createTypeCommand(selections: SelectionSet, text: string): EditorLanguageTypeCommand | undefined;
	createEnterCommand(selections: SelectionSet): EditorEditCommand | undefined;
	createBackspaceCommand(selections: SelectionSet): EditorEditCommand | undefined;
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
	readonly userInputEvents?: EditorViewUserInputEvents;
}

/**
 * Routes semantic editor commands into common editing operations.
 *
 * This is the Stanza equivalent of VS Code's EditorViewInputController: browser input
 * adapters normalize raw events, while this class owns command execution,
 * command transformation, overtype, and contribution-facing edit events.
 */
export class EditorViewInputController extends Disposable {
	private readonly didChangeOvertypeEmitter = this._register(new Emitter<boolean>());
	private readonly didEditEmitter = this._register(new Emitter<EditorViewDidEditEvent>());
	private readonly commandTransformers: EditorCommandTransformer[] = [];
	private readonly languageEditing: EditorLanguageEditingAdapter | undefined;
	private readonly wordPattern: (() => RegExp | undefined) | undefined;
	private readonly userInputEvents: EditorViewUserInputEvents;
	private overtype = false;

	readonly onDidChangeOvertype: Event<boolean> = this.didChangeOvertypeEmitter.event;
	readonly onDidEdit: Event<EditorViewDidEditEvent> = this.didEditEmitter.event;

	constructor(
		private readonly viewport: View,
		private readonly selectionController: CursorsController,
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
			this.userInputEvents = options.userInputEvents ?? new EditorViewUserInputEvents();
		} catch (error) {
			this.dispose();
			throw error;
		}
	}

	get overtyping(): boolean {
		return this.overtype;
	}

	get hasExpandedSelections(): boolean {
		return this.selectionController.selections.selections.some(selection => !selection.isEmpty());
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
			this.languageEditing?.createBackspaceCommand(this.selectionController.selections) ?? SelectionSetDeleteOperations.deleteLeft(this.viewport.textModel, this.selectionController.selections),
			inputType,
		);
	}

	public deleteForward(inputType = 'deleteContentForward'): TextModelChange | undefined {
		return this.execute(SelectionSetDeleteOperations.deleteRight(this.viewport.textModel, this.selectionController.selections), inputType);
	}

	public deleteWordBackward(inputType = 'deleteWordBackward'): TextModelChange | undefined {
		return this.execute(SelectionSetWordOperations.deleteWordLeft(this.viewport.textModel, this.selectionController.selections, this.currentWordPattern), inputType);
	}

	public deleteWordForward(inputType = 'deleteWordForward'): TextModelChange | undefined {
		return this.execute(SelectionSetWordOperations.deleteWordRight(this.viewport.textModel, this.selectionController.selections, this.currentWordPattern), inputType);
	}

	public deleteSoftLineBackward(inputType = 'deleteSoftLineBackward'): TextModelChange | undefined {
		return this.execute(SelectionSetDeleteOperations.deleteToBeginningOfLine(this.viewport.textModel, this.selectionController.selections), inputType);
	}

	public deleteSoftLineForward(inputType = 'deleteSoftLineForward'): TextModelChange | undefined {
		return this.execute(SelectionSetDeleteOperations.deleteToEndOfLine(this.viewport.textModel, this.selectionController.selections), inputType);
	}

	public insertTab(): TextModelChange | undefined {
		return this.execute(TypeOperations.typeWithoutInterceptors(this.viewport.textModel, this.selectionController.selections, '\t'), 'insertText', '\t', undefined, false);
	}

	public applyTextUpdate(update: EditContextTextUpdate): TextModelChange | undefined {
		const model = this.viewport.textModel;
		const selections = this.selectionsForTextUpdate(update);
		const inputType = update.inputType ?? (update.text.length > 0 ? 'insertText' : 'deleteContentBackward');
		if (update.text.length > 0) {
			if (inputType === 'insertLineBreak' || inputType === 'insertParagraph') return this.executeEnter(selections, inputType, update.text);
			return this.executeType(selections, update.text, inputType);
		}
		if (inputType === 'deleteContentForward') return this.execute(SelectionSetDeleteOperations.deleteRight(model, selections), inputType);
		if (inputType === 'deleteContentBackward') {
			return this.execute(
				this.languageEditing?.createBackspaceCommand(selections) ?? SelectionSetDeleteOperations.deleteLeft(model, selections),
				inputType,
			);
		}
		return this.execute(TypeOperations.typeWithoutInterceptors(model, selections, ''), inputType);
	}

	public undo(): void {
		this.selectionController.undo();
		this.revealPrimary();
	}

	public redo(): void {
		this.selectionController.redo();
		this.revealPrimary();
	}

	private executeType(selections: SelectionSet, text: string, inputType: string): TextModelChange | undefined {
		const languageTypeCommand = this.languageEditing?.createTypeCommand(selections, text);
		const insertedText = languageTypeCommand?.insertedText === false ? undefined : text;
		const command = languageTypeCommand?.command ?? (this.overtype
			? AutoClosingOvertypeOperation.getEdits(this.viewport.textModel, selections, text)
			: TypeOperations.typeWithoutInterceptors(this.viewport.textModel, selections, text));
		return this.execute(command, inputType, insertedText, languageTypeCommand?.afterExecute);
	}

	private executeEnter(selections: SelectionSet, inputType: string, text = '\n'): TextModelChange | undefined {
		const command = this.languageEditing?.createEnterCommand(selections) ?? TypeOperations.typeWithoutInterceptors(this.viewport.textModel, selections, text);
		return this.execute(command, inputType);
	}

	private selectionsForTextUpdate(update: EditContextTextUpdate): SelectionSet {
		const current = this.selectionController.selections;
		const primary = current.primary;
		const primaryStart = this.viewport.textModel.offsetAt(primary.getStartPosition());
		const primaryEnd = this.viewport.textModel.offsetAt(primary.getEndPosition());
		if (primaryStart === update.previousSelectionStart && primaryEnd === update.previousSelectionEnd) return current;
		return SelectionSet.single(Selection.fromPositions(
			this.viewport.textModel.positionAt(update.updateRangeStart),
			this.viewport.textModel.positionAt(update.updateRangeEnd),
		));
	}

	private execute(command: EditorEditCommand, inputType: string, insertedText: string | undefined = undefined, afterExecute?: (change: TextModelChange) => void, emitDidEdit = true): TextModelChange | undefined {
		for (const transformer of this.commandTransformers) command = transformer(command, { inputType });
		const change = this.selectionController.execute(command);
		this.revealPrimary();
		if (change) {
			afterExecute?.(change);
			if (emitDidEdit) this.didEditEmitter.fire(Object.freeze({ inputType, insertedText, change }));
		}
		return change;
	}

	private revealPrimary(): void {
		this.viewport.revealPosition(this.selectionController.selections.primary.getPosition());
	}

	private get currentWordPattern(): RegExp | undefined {
		return this.wordPattern?.();
	}

	/** Forwards view-originated input without taking ownership of its policy. */
	emitKeyDown(event: IKeyboardEvent): void {
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

/** Browser input adapter for DOM-free language editing commands. */
export class LanguageEditingAdapter extends Disposable implements EditorLanguageEditingAdapter {
	private readonly autoClosingTracker: LanguageAutoClosingTracker;
	private readonly lexicalContext: LanguageLexicalContextSource;

	constructor(readonly textModel: TextModel, private readonly selections: CursorsController, private readonly languageId: string, private readonly configurations: ILanguageConfigurationService, lexicalContext: LanguageLexicalContextSource | undefined = undefined, private readonly indentation: EditorIndentationOptions | undefined = undefined) {
		super();
		assertLanguageId(languageId);
		if (!configurations || typeof configurations.getLanguageConfiguration !== "function") throw new TypeError("Stanza text input language requires a configuration source");
		resolveEditorIndentationOptions(indentation);
		if (lexicalContext && (lexicalContext.textModel !== textModel || lexicalContext.languageId !== languageId)) throw new TypeError("Stanza text input lexical context must match its model and language");
		this.lexicalContext = lexicalContext ?? this._register(new LanguageLexicalContextIndex(textModel, languageId, configurations));
		this.autoClosingTracker = this._register(new LanguageAutoClosingTracker(textModel, selections));
	}

	createTypeCommand(selections: SelectionSet, text: string): EditorLanguageTypeCommand | undefined {
		const result = createLanguagePairTypeCommand(this.textModel, selections, text, this.configurationAt(selections.primary.getPosition()), { autoClosingTrust: this.autoClosingTracker, lexicalContext: this.lexicalContext });
		if (!result) return undefined;
		return Object.freeze({
			command: result.command,
			insertedText: result.didInsertText,
			afterExecute: (change: TextModelChange) => {
				if (result.autoClosingActions.length > 0) this.autoClosingTracker.record(result.autoClosingActions, change.version);
			},
		});
	}

	createEnterCommand(selections: SelectionSet): EditorEditCommand {
		return createLanguageEnterCommand(this.textModel, selections, this.configurationAt(selections.primary.getPosition()), { indentation: this.indentation, lexicalContext: this.lexicalContext });
	}

	createBackspaceCommand(selections: SelectionSet): EditorEditCommand | undefined {
		return createLanguagePairBackspaceCommand(this.textModel, selections, this.configurationAt(selections.primary.getPosition()), this.autoClosingTracker);
	}

	private configurationAt(position: Position) {
		return this.configurations.getLanguageConfiguration(this.lexicalContext.getLanguageIdAt(position));
	}
}

export interface KeyboardNavigationControllerOptions {
	readonly operatingSystem?: OperatingSystem;
	/** Resolves the active language word matcher for word navigation. */
	readonly wordPattern?: () => RegExp | undefined;
	readonly stickyTabStops?: boolean;
	readonly tabSize?: number;
}

export interface KeyboardNavigationCommand {
	readonly command: EditorCursorNavigationCommand;
	readonly mode: EditorCursorNavigationMode;
}

/**
 * Routes browser keydown navigation into Stanza common selection commands.
 */
export class KeyboardNavigationController extends Disposable {
	private readonly targetOperatingSystem: OperatingSystem;
	private readonly wordPattern: (() => RegExp | undefined) | undefined;
	private readonly atomicTabSize: number | undefined;
	private preferredColumns: readonly number[] | undefined;
	private preferredVisualHorizontalOffsets: readonly number[] | undefined;
	private applyingNavigation = false;

	constructor(
		private readonly viewport: View,
		private readonly selectionController: CursorsController,
		userInputEvents: EditorViewUserInputEvents,
		options: KeyboardNavigationControllerOptions = {},
	) {
		super();
		try {
			this.targetOperatingSystem = readOperatingSystem(
				options.operatingSystem,
			);
			if (options.wordPattern !== undefined && typeof options.wordPattern !== "function") {
				throw new TypeError("Stanza keyboard word pattern resolver must be a function");
			}
			if (options.stickyTabStops !== undefined && typeof options.stickyTabStops !== 'boolean') {
				throw new TypeError('Stanza sticky tab stops must be boolean');
			}
			if (options.tabSize !== undefined && (!Number.isSafeInteger(options.tabSize) || options.tabSize < 1)) {
				throw new RangeError('Stanza keyboard tab size must be a positive safe integer');
			}
			this.wordPattern = options.wordPattern;
			this.atomicTabSize = options.stickyTabStops ? options.tabSize ?? 4 : undefined;
		} catch (error) {
			this.dispose();
			throw error;
		}
		if (viewport.textModel !== selectionController.textModel) {
			this.dispose();
			throw new TypeError(
				"Stanza keyboard and selection controllers must share one text model",
			);
		}
		const previousKeyDownHandler = userInputEvents.onKeyDown;
		const keyDownHandler = (event: IKeyboardEvent): void => {
			previousKeyDownHandler?.(event);
			if (!event.browserEvent.defaultPrevented) {
				this.handleKeyDown(event);
			}
		};
		userInputEvents.onKeyDown = keyDownHandler;
		this._register(toDisposable(() => {
			if (userInputEvents.onKeyDown === keyDownHandler) {
				userInputEvents.onKeyDown = previousKeyDownHandler;
			}
		}));
		this._register(selectionController.onDidChange(() => {
			if (!this.applyingNavigation) {
				this.preferredColumns = undefined;
				this.preferredVisualHorizontalOffsets = undefined;
			}
		}));
	}

	private handleKeyDown(event: IKeyboardEvent): void {
		const navigation = resolveStanzaKeyboardNavigation(
			event,
			this.targetOperatingSystem,
		);
		if (!navigation) return;
		event.stop();
		const layout = this.viewport.viewportLayout;
		const pageLineCount = Math.max(
			1,
			Math.floor(layout.viewportSize.height / layout.lineHeight),
		);
		const visualCommand = isVisualVerticalCommand(navigation.command)
			? navigation.command
			: undefined;
		const result = this.viewport.lineWrapping === EditorLineWrapping.On &&
			visualCommand !== undefined
			? navigateStanzaVisualCursors(
				this.viewport.textModel,
				this.viewport.getVisualLineProjection(),
				this.selectionController.selections,
				{
					command: visualCommand,
					mode: navigation.mode,
					pageLineCount,
					preferredHorizontalOffsets: this.preferredVisualHorizontalOffsets,
				},
				text => this.viewport.measureTextWidth(text),
				{
					getHorizontalOffset: position => this.viewport.getVisualHorizontalOffset(position),
					getNearestPosition: (visualLineIndex, horizontalOffset) => this.viewport.getNearestPositionAtVisualHorizontalOffset(visualLineIndex, horizontalOffset),
				},
			)
			: CursorNavigation.navigate(
				this.viewport.textModel,
				this.selectionController.selections,
				{
					...navigation,
					pageLineCount,
					...(this.wordPattern ? { wordPattern: this.wordPattern() } : {}),
					...(this.atomicTabSize === undefined ? {} : { atomicTabSize: this.atomicTabSize }),
					preferredColumns: this.preferredColumns,
				},
			);
		this.applyingNavigation = true;
		try {
			this.selectionController.setSelections(result.selections);
		} finally {
			this.applyingNavigation = false;
		}
		if ("preferredHorizontalOffsets" in result) {
			this.preferredColumns = undefined;
			this.preferredVisualHorizontalOffsets = result.preferredHorizontalOffsets;
		} else {
			this.preferredColumns = result.preferredColumns;
			this.preferredVisualHorizontalOffsets = undefined;
		}
		this.viewport.revealPosition(result.selections.primary.getPosition());
	}
}

function isVisualVerticalCommand(command: EditorCursorNavigationCommand): command is EditorCursorNavigationCommand.LineUp | EditorCursorNavigationCommand.LineDown | EditorCursorNavigationCommand.PageUp | EditorCursorNavigationCommand.PageDown {
	return command === EditorCursorNavigationCommand.LineUp ||
		command === EditorCursorNavigationCommand.LineDown ||
		command === EditorCursorNavigationCommand.PageUp ||
		command === EditorCursorNavigationCommand.PageDown;
}

export function resolveStanzaKeyboardNavigation(event: Pick<IKeyboardEvent, "key" | "ctrlKey" | "shiftKey" | "altKey" | "metaKey" | "altGraphKey" | "isComposing">, targetOperatingSystem: OperatingSystem): KeyboardNavigationCommand | undefined {
	if (event.isComposing || event.altGraphKey) return undefined;
	const mode = event.shiftKey
		? EditorCursorNavigationMode.Extend
		: EditorCursorNavigationMode.Move;
	const noCommandModifier =
		!event.ctrlKey && !event.altKey && !event.metaKey;
	if (noCommandModifier) {
		const command = unmodifiedCommand(event.key);
		return command ? { command, mode } : undefined;
	}

	if (targetOperatingSystem === OperatingSystem.Macintosh) {
		if (event.altKey && !event.ctrlKey && !event.metaKey) {
			if (event.key === "ArrowLeft") {
				return { command: EditorCursorNavigationCommand.WordLeft, mode };
			}
			if (event.key === "ArrowRight") {
				return { command: EditorCursorNavigationCommand.WordRight, mode };
			}
		}
		if (event.metaKey && !event.ctrlKey && !event.altKey) {
			const command = macCommandCommand(event.key);
			return command ? { command, mode } : undefined;
		}
		return undefined;
	}

	if (event.ctrlKey && !event.altKey && !event.metaKey) {
		const command = controlCommand(event.key);
		return command ? { command, mode } : undefined;
	}
	return undefined;
}

function unmodifiedCommand(key: string): EditorCursorNavigationCommand | undefined {
	switch (key) {
		case "ArrowLeft":
			return EditorCursorNavigationCommand.CharacterLeft;
		case "ArrowRight":
			return EditorCursorNavigationCommand.CharacterRight;
		case "ArrowUp":
			return EditorCursorNavigationCommand.LineUp;
		case "ArrowDown":
			return EditorCursorNavigationCommand.LineDown;
		case "Home":
			return EditorCursorNavigationCommand.LineStart;
		case "End":
			return EditorCursorNavigationCommand.LineEnd;
		case "PageUp":
			return EditorCursorNavigationCommand.PageUp;
		case "PageDown":
			return EditorCursorNavigationCommand.PageDown;
		default:
			return undefined;
	}
}

function controlCommand(key: string): EditorCursorNavigationCommand | undefined {
	switch (key) {
		case "ArrowLeft":
			return EditorCursorNavigationCommand.WordLeft;
		case "ArrowRight":
			return EditorCursorNavigationCommand.WordRight;
		case "Home":
			return EditorCursorNavigationCommand.DocumentStart;
		case "End":
			return EditorCursorNavigationCommand.DocumentEnd;
		default:
			return undefined;
	}
}

function macCommandCommand(key: string): EditorCursorNavigationCommand | undefined {
	switch (key) {
		case "ArrowLeft":
			return EditorCursorNavigationCommand.LineStart;
		case "ArrowRight":
			return EditorCursorNavigationCommand.LineEnd;
		case "ArrowUp":
		case "Home":
			return EditorCursorNavigationCommand.DocumentStart;
		case "ArrowDown":
		case "End":
			return EditorCursorNavigationCommand.DocumentEnd;
		default:
			return undefined;
	}
}

function readOperatingSystem(value: OperatingSystem | undefined): OperatingSystem {
	const resolved = value ?? operatingSystem;
	if (!Object.values(OperatingSystem).includes(resolved)) {
		throw new TypeError("Unknown Stanza keyboard operating system");
	}
	return resolved;
}
