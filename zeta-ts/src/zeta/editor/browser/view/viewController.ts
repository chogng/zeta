import { type IKeyboardEvent } from '../../../base/browser/keyboardEvent.js';
import { type IMouseWheelEvent } from '../../../base/browser/mouseEvent.js';
import { addDisposableListener, getClientArea } from '../../../base/browser/dom.js';
import { Emitter, type Event } from '../../../base/common/event.js';
import { Disposable, toDisposable, type IDisposable } from '../../../base/common/lifecycle.js';
import { isLinux, operatingSystem, OperatingSystem } from '../../../base/common/platform.js';
import { type EditorEditCommand } from '../../common/commands/editorEditCommand.js';
import { EditorLineWrapping, EditorOption } from '../../common/config/editorOptions.js';
import { ColumnSelection } from '../../common/cursor/cursorColumnSelection.js';
import { CursorMoveCommands } from '../../common/cursor/cursorMoveCommands.js';
import { EditorCursorNavigationCommand, EditorCursorNavigationMode, MoveOperations } from '../../common/cursor/cursorMoveOperations.js';
import { DeleteOperations } from '../../common/cursor/cursorDeleteOperations.js';
import { WordOperations } from '../../common/cursor/cursorWordOperations.js';
import { type CursorsController } from '../../common/cursor/cursor.js';
import { AutoClosingOvertypeOperation } from '../../common/cursor/cursorTypeEditOperations.js';
import { TypeOperations } from '../../common/cursor/cursorTypeOperations.js';
import { Selection } from '../../common/core/selection.js';
import { Position } from '../../common/core/position.js';
import { type IDimension } from '../../common/core/2d/dimension.js';
import { Range } from '../../common/core/range.js';
import { type TextModelChange } from '../../common/core/textChange.js';
import { resolveEditorIndentationOptions, type EditorIndentationOptions } from '../../common/core/misc/indentation.js';
import { type ILanguageConfigurationService } from '../../common/languages/languageConfigurationRegistry.js';
import { type LanguageLexicalContextSource, LanguageLexicalContextIndex } from '../../common/languages/languageLexicalContext.js';
import { assertLanguageId } from '../../common/languages/languageId.js';
import { type TextModel } from '../../common/model/textModel.js';
import { navigateStanzaVisualCursors } from '../../common/viewModel/visualCursorNavigation.js';
import { type View } from '../view.js';
import { NavigationCommandRevealType } from '../coreCommands.js';
import { type AbstractEditContext, type CompositionController, type EditContextCharacterBounds, type EditContextOptions, type EditContextTextUpdate } from '../controller/editContext/editContext.js';
import { NativeEditContext, type NativeEditContextWindow } from '../controller/editContext/native/nativeEditContext.js';
import { TextAreaEditContext } from '../controller/editContext/textArea/textAreaEditContext.js';
import { ViewUserInputEvents } from './viewUserInputEvents.js';
import { type IAccessibilityService } from '../../../platform/accessibility/common/accessibility.js';
import { type IEditorAriaOptions, type IEditorMouseEvent, type IPartialEditorMouseEvent } from '../editorBrowser.js';
import { type BracketColorizationSource, type SemanticTokenSource } from '../viewParts/viewLines/viewLine.js';

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
	createTypeCommand(selections: readonly Selection[], text: string): EditorLanguageTypeCommand | undefined;
	createEnterCommand(selections: readonly Selection[]): EditorEditCommand | undefined;
	createBackspaceCommand(selections: readonly Selection[]): EditorEditCommand | undefined;
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
	readonly ownerId?: string;
	readonly ariaLabel?: string;
	readonly accessibilityService?: IAccessibilityService;
	readonly renderRichScreenReaderContent?: boolean;
	readonly accessibilityPageSize?: number;
	readonly semanticTokenSource?: SemanticTokenSource;
	readonly bracketColorizationSource?: BracketColorizationSource;
	readonly languageEditing?: EditorLanguageEditingAdapter;
	readonly wordPattern?: () => RegExp | undefined;
	readonly userInputEvents?: ViewUserInputEvents;
}

export interface IMouseDispatchData {
	position: Position;
	mouseColumn: number;
	revealType: NavigationCommandRevealType;
	startedOnLineNumbers: boolean;
	inSelectionMode: boolean;
	mouseDownCount: number;
	altKey: boolean;
	ctrlKey: boolean;
	metaKey: boolean;
	shiftKey: boolean;
	leftButton: boolean;
	middleButton: boolean;
	onInjectedText: boolean;
}

const enum MouseSelectionKind {
	Character,
	Word,
	WholeLine,
	ExtendToWord,
	ExtendToLine,
	Column,
}

interface MouseSelectionState {
	readonly kind: MouseSelectionKind;
	readonly anchorRange: Range;
	readonly baseSelections: readonly Selection[] | undefined;
	readonly toggleCandidateIndex: number | undefined;
}

/**
 * Routes semantic editor commands into common editing operations.
 *
 * Browser input adapters normalize raw events, while this class owns their lifecycle,
 * command execution, command transformation, overtype, and contribution-facing events.
 */
export class ViewController extends Disposable {
	private readonly didChangeOvertypeEmitter = this._register(new Emitter<boolean>());
	private readonly didEditEmitter = this._register(new Emitter<EditorViewDidEditEvent>());
	private readonly commandTransformers: EditorCommandTransformer[] = [];
	private readonly languageEditing: EditorLanguageEditingAdapter | undefined;
	private readonly wordPattern: (() => RegExp | undefined) | undefined;
	readonly userInputEvents: ViewUserInputEvents;
	readonly ownerId: string;
	readonly editContext: AbstractEditContext;
	readonly element: HTMLElement;
	readonly textArea: HTMLTextAreaElement | undefined;
	readonly compositionController: CompositionController;
	readonly onWillBeforeInput: Event<InputEvent>;
	readonly onWillTextUpdate: Event<EditorViewTextUpdateEvent>;
	readonly onWillKeydown: Event<KeyboardEvent>;
	private overtype = false;
	private mouseSelection: MouseSelectionState | undefined;

	readonly onDidChangeOvertype: Event<boolean> = this.didChangeOvertypeEmitter.event;
	readonly onDidEdit: Event<EditorViewDidEditEvent> = this.didEditEmitter.event;

	constructor(
		readonly viewport: View,
		readonly selectionController: CursorsController,
		options: ViewControllerOptions = {},
	) {
		super();
		try {
			validateAccessibilityPageSize(options.accessibilityPageSize);
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
			this.userInputEvents = options.userInputEvents ?? new ViewUserInputEvents(viewport.coordinatesConverter);
			this.ownerId = options.ownerId === undefined ? nextViewId() : validateOwnerId(options.ownerId);
			this.editContext = this._register(createEditContext(viewport.element, {
				ariaLabel: options.ariaLabel,
				readOnly: selectionController.readOnly,
				textDirection: viewport.editorTextDirection,
				ownerId: this.ownerId,
				characterBoundsProvider: modelOffset => this.characterBoundsAt(modelOffset),
				viewController: this,
				viewport,
				selectionController,
				accessibilityService: options.accessibilityService,
				renderRichScreenReaderContent: options.renderRichScreenReaderContent,
				accessibilityPageSize: options.accessibilityPageSize,
				semanticTokenSource: options.semanticTokenSource,
				bracketColorizationSource: options.bracketColorizationSource,
			}));
			this.element = this.editContext.domNode;
			this.textArea = this.editContext instanceof TextAreaEditContext ? this.editContext.getTextAreaDomNode() : undefined;
			this.compositionController = this.editContext.compositionController;
			this.onWillBeforeInput = this.editContext.onWillBeforeInput;
			this.onWillTextUpdate = this.editContext.onWillTextUpdate;
			this.onWillKeydown = this.editContext.onWillKeydown;
			this._register(this.onDidChangeOvertype(overtyping => {
				viewport.element.classList.toggle('overtype', overtyping);
				viewport.setOvertype(overtyping);
			}));
			this._register(this.compositionController.onDidChange(composing => {
				if (!composing) this.synchronizeEditContext();
			}));
			this._register(toDisposable(() => {
				viewport.element.classList.remove('input-focused');
				viewport.element.classList.remove('overtype');
				viewport.setOvertype(false);
			}));
			this._register(addDisposableListener(viewport.element, 'focus', event => {
				if (event.target === viewport.element) this.focus();
			}));
			this._register(this.editContext.onDidFocus(() => viewport.element.classList.add('input-focused')));
			this._register(this.editContext.onDidBlur(() => {
				viewport.element.classList.remove('input-focused');
				this.editContext.clear();
			}));
			this._register(selectionController.onDidChange(() => this.synchronizeEditContext()));
			this._register(viewport.textModel.onDidChangeContent(() => this.synchronizeEditContext()));
			this.synchronizeEditContext();
			this.editContext.connect();
		} catch (error) {
			this.dispose();
			throw error;
		}
	}

	layout(dimension: IDimension = getClientArea(this.viewport.element)): void {
		this.viewport.layout({ width: Math.max(0, dimension.width), height: Math.max(0, dimension.height) });
	}

	focus(): void { this.editContext.focus(); }
	isFocused(): boolean { return this.editContext.isFocused(); }
	refreshFocusState(): void { this.editContext.refreshFocusState(); }
	setAriaOptions(options: IEditorAriaOptions): void { this.editContext.setAriaOptions(options); }
	writeScreenReaderContent(reason: string): void { this.editContext.writeScreenReaderContent(reason); }
	revealPosition(position: Position): void { this.viewport.revealPosition(position); }
	clearInput(): void { this.editContext.clear(); }

	get overtyping(): boolean {
		return this.overtype;
	}

	get hasExpandedSelections(): boolean {
		return this.selectionController.selections.some(selection => !selection.isEmpty());
	}

	public setSelection(modelSelection: Selection): void {
		this.viewport.textModel.validateRange(modelSelection);
		this.selectionController.setSelections([modelSelection]);
		this.viewport.revealPosition(modelSelection.getPosition());
	}

	public moveTo(viewPosition: Position, revealType: NavigationCommandRevealType): void {
		const position = this.viewport.coordinatesConverter.convertViewPositionToModelPosition(viewPosition);
		this.selectionController.setSelections([Selection.fromPositions(position)]);
		this.revealMousePosition(position, revealType);
	}

	public dispatchMouse(data: IMouseDispatchData): void {
		const position = this.viewport.coordinatesConverter.convertViewPositionToModelPosition(data.position);
		if (data.inSelectionMode && this.mouseSelection) {
			this.applyMouseSelection(this.mouseSelection, position, data.revealType);
			return;
		}

		this.mouseSelection = undefined;
		const options = this.viewport;
		const selectionClipboard = isLinux && options.getOption(EditorOption.selectionClipboard);
		if (data.middleButton && !selectionClipboard) {
			if (!options.getOption(EditorOption.scrollOnMiddleClick)) {
				this.beginMouseSelection(MouseSelectionKind.Column, position, data, false);
			}
			return;
		}

		const multiCursor = this.hasMultiCursorModifier(data);
		if (data.startedOnLineNumbers) {
			this.beginMouseSelection(data.shiftKey ? MouseSelectionKind.ExtendToLine : MouseSelectionKind.WholeLine, position, data, multiCursor);
			return;
		}
		if (data.mouseDownCount >= 4) {
			const model = this.viewport.textModel;
			this.setSelection(Selection.fromPositions(model.positionAt(0), model.positionAt(model.length)));
			return;
		}
		if (data.mouseDownCount === 3) {
			this.beginMouseSelection(data.shiftKey ? MouseSelectionKind.ExtendToLine : MouseSelectionKind.WholeLine, position, data, multiCursor);
			return;
		}
		if (data.mouseDownCount === 2) {
			if (!data.onInjectedText) {
				this.beginMouseSelection(data.shiftKey ? MouseSelectionKind.ExtendToWord : MouseSelectionKind.Word, position, data, multiCursor);
			}
			return;
		}

		if (multiCursor) {
			if (this.hasNonMultiCursorModifier(data)) return;
			this.beginMouseSelection(data.shiftKey ? MouseSelectionKind.Column : MouseSelectionKind.Character, position, data, !data.shiftKey);
			return;
		}
		if (data.altKey || options.getOption(EditorOption.columnSelection)) {
			this.beginMouseSelection(MouseSelectionKind.Column, position, data, false);
			return;
		}
		this.beginMouseSelection(MouseSelectionKind.Character, position, data, false);
	}

	private beginMouseSelection(kind: MouseSelectionKind, position: Position, data: IMouseDispatchData, addSelection: boolean): void {
		const primary = this.selectionController.selections[0]!;
		const extend = data.shiftKey || data.inSelectionMode;
		let anchorRange: Range;
		switch (kind) {
			case MouseSelectionKind.Word:
				anchorRange = WordOperations.getWordSelectionRange(this.viewport.textModel, position, this.currentWordPattern);
				break;
			case MouseSelectionKind.WholeLine:
				anchorRange = Range.fromPositions(lineStart(position.lineNumber));
				break;
			case MouseSelectionKind.ExtendToWord:
			case MouseSelectionKind.ExtendToLine:
				anchorRange = Range.fromPositions(primary.getSelectionStart());
				break;
			case MouseSelectionKind.Column:
			case MouseSelectionKind.Character:
				anchorRange = Range.fromPositions(extend ? primary.getSelectionStart() : position);
				break;
		}
		const initialSelection = selectionForMouseTarget(kind, this.viewport.textModel, anchorRange, position, this.currentWordPattern);
		const baseSelections = addSelection ? Object.freeze([...this.selectionController.selections]) : undefined;
		this.mouseSelection = {
			kind,
			anchorRange,
			baseSelections,
			toggleCandidateIndex: baseSelections ? CursorMoveCommands.findPointerToggleCandidate(baseSelections, initialSelection) : undefined,
		};
		this.applyMouseSelection(this.mouseSelection, position, data.revealType);
	}

	private applyMouseSelection(state: MouseSelectionState, position: Position, revealType: NavigationCommandRevealType): void {
		if (state.kind === MouseSelectionKind.Column) {
			this.selectionController.setSelections(ColumnSelection.columnSelect(this.viewport.textModel, state.anchorRange.getStartPosition(), position));
			this.revealMousePosition(position, revealType);
			return;
		}
		const selection = selectionForMouseTarget(state.kind, this.viewport.textModel, state.anchorRange, position, this.currentWordPattern);
		this.selectionController.setSelections(state.baseSelections
			? CursorMoveCommands.combinePointerSelection(state.baseSelections, selection, state.toggleCandidateIndex)
			: [selection]);
		this.revealMousePosition(position, revealType);
	}

	private revealMousePosition(position: Position, revealType: NavigationCommandRevealType): void {
		if (revealType !== NavigationCommandRevealType.None) this.viewport.revealPosition(position);
	}

	private hasMultiCursorModifier(data: IMouseDispatchData): boolean {
		return data[this.viewport.getOption(EditorOption.multiCursorModifier)];
	}

	private hasNonMultiCursorModifier(data: IMouseDispatchData): boolean {
		switch (this.viewport.getOption(EditorOption.multiCursorModifier)) {
			case 'altKey': return data.ctrlKey || data.metaKey;
			case 'ctrlKey': return data.altKey || data.metaKey;
			case 'metaKey': return data.ctrlKey || data.altKey;
		}
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
			this.languageEditing?.createBackspaceCommand(this.selectionController.selections) ?? DeleteOperations.deleteLeft(this.viewport.textModel, this.selectionController.selections),
			inputType,
		);
	}

	public deleteForward(inputType = 'deleteContentForward'): TextModelChange | undefined {
		return this.execute(DeleteOperations.deleteRight(this.viewport.textModel, this.selectionController.selections), inputType);
	}

	public deleteWordBackward(inputType = 'deleteWordBackward'): TextModelChange | undefined {
		return this.execute(WordOperations.deleteWordLeft(this.viewport.textModel, this.selectionController.selections, this.currentWordPattern), inputType);
	}

	public deleteWordForward(inputType = 'deleteWordForward'): TextModelChange | undefined {
		return this.execute(WordOperations.deleteWordRight(this.viewport.textModel, this.selectionController.selections, this.currentWordPattern), inputType);
	}

	public deleteSoftLineBackward(inputType = 'deleteSoftLineBackward'): TextModelChange | undefined {
		return this.execute(DeleteOperations.deleteToBeginningOfLine(this.viewport.textModel, this.selectionController.selections), inputType);
	}

	public deleteSoftLineForward(inputType = 'deleteSoftLineForward'): TextModelChange | undefined {
		return this.execute(DeleteOperations.deleteToEndOfLine(this.viewport.textModel, this.selectionController.selections), inputType);
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
		if (inputType === 'deleteContentForward') return this.execute(DeleteOperations.deleteRight(model, selections), inputType);
		if (inputType === 'deleteContentBackward') {
			return this.execute(
				this.languageEditing?.createBackspaceCommand(selections) ?? DeleteOperations.deleteLeft(model, selections),
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

	private executeType(selections: readonly Selection[], text: string, inputType: string): TextModelChange | undefined {
		const languageTypeCommand = this.languageEditing?.createTypeCommand(selections, text);
		const insertedText = languageTypeCommand?.insertedText === false ? undefined : text;
		const command = languageTypeCommand?.command ?? (this.overtype
			? AutoClosingOvertypeOperation.getEdits(this.viewport.textModel, selections, text)
			: TypeOperations.typeWithoutInterceptors(this.viewport.textModel, selections, text));
		return this.execute(command, inputType, insertedText, languageTypeCommand?.afterExecute);
	}

	private executeEnter(selections: readonly Selection[], inputType: string, text = '\n'): TextModelChange | undefined {
		const command = this.languageEditing?.createEnterCommand(selections) ?? TypeOperations.typeWithoutInterceptors(this.viewport.textModel, selections, text);
		return this.execute(command, inputType);
	}

	private selectionsForTextUpdate(update: EditContextTextUpdate): readonly Selection[] {
		const current = this.selectionController.selections;
		const primary = current[0]!;
		const primaryStart = this.viewport.textModel.offsetAt(primary.getStartPosition());
		const primaryEnd = this.viewport.textModel.offsetAt(primary.getEndPosition());
		if (primaryStart === update.previousSelectionStart && primaryEnd === update.previousSelectionEnd) return current;
		return [Selection.fromPositions(
			this.viewport.textModel.positionAt(update.updateRangeStart),
			this.viewport.textModel.positionAt(update.updateRangeEnd),
		)];
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
		this.viewport.revealPosition(this.selectionController.selections[0]!.getPosition());
	}

	private get currentWordPattern(): RegExp | undefined {
		return this.wordPattern?.();
	}

	/** Forwards view-originated input without taking ownership of its policy. */
	emitKeyDown(event: IKeyboardEvent): void {
		this.userInputEvents.emitKeyDown(event);
	}

	emitKeyUp(event: IKeyboardEvent): void {
		this.userInputEvents.emitKeyUp(event);
	}

	emitContextMenu(event: IEditorMouseEvent): void {
		this.userInputEvents.emitContextMenu(event);
	}

	emitMouseMove(event: IEditorMouseEvent): void {
		this.userInputEvents.emitMouseMove(event);
	}

	emitMouseLeave(event: IPartialEditorMouseEvent): void {
		this.userInputEvents.emitMouseLeave(event);
	}

	emitMouseDown(event: IEditorMouseEvent): void {
		this.userInputEvents.emitMouseDown(event);
	}

	emitMouseUp(event: IEditorMouseEvent): void {
		this.userInputEvents.emitMouseUp(event);
	}

	emitMouseDrag(event: IEditorMouseEvent): void {
		this.userInputEvents.emitMouseDrag(event);
	}

	emitMouseDrop(event: IPartialEditorMouseEvent): void {
		this.userInputEvents.emitMouseDrop(event);
	}

	emitMouseDropCanceled(): void {
		this.userInputEvents.emitMouseDropCanceled();
	}

	emitMouseWheel(event: IMouseWheelEvent): void {
		this.userInputEvents.emitMouseWheel(event);
	}

	private synchronizeEditContext(): void {
		const selection = this.selectionController.selections[0]!;
		this.editContext.syncState({
			text: this.viewport.textModel.getText(),
			selectionStart: this.viewport.textModel.offsetAt(selection.getStartPosition()),
			selectionEnd: this.viewport.textModel.offsetAt(selection.getEndPosition()),
			position: selection.getPosition(),
		});
		this.editContext.updateBounds(this.viewport.getPositionContentCoordinates(selection.getPosition()));
		this.editContext.writeScreenReaderContent('editor state changed');
	}

	private characterBoundsAt(modelOffset: number): EditContextCharacterBounds | undefined {
		const model = this.viewport.textModel;
		if (!Number.isSafeInteger(modelOffset) || modelOffset < 0 || modelOffset >= model.length) return undefined;
		const position = model.positionAt(modelOffset);
		const next = model.positionAt(Math.min(model.length, modelOffset + 1));
		const start = this.viewport.getPositionContentCoordinates(position);
		const end = this.viewport.getPositionContentCoordinates(next);
		return Object.freeze({
			left: Math.min(start.left, end.left),
			top: start.top,
			width: position.lineNumber === next.lineNumber ? Math.max(1, Math.abs(end.left - start.left)) : Math.max(1, this.viewport.measureTextWidth(' ')),
			height: start.height,
		});
	}
}

function selectionForMouseTarget(kind: MouseSelectionKind, model: TextModel, anchorRange: Range, position: Position, wordPattern: RegExp | undefined): Selection {
	const anchor = anchorRange.getStartPosition();
	switch (kind) {
		case MouseSelectionKind.Character:
			return Selection.fromPositions(anchor, position);
		case MouseSelectionKind.Column:
			return Selection.fromPositions(anchor);
		case MouseSelectionKind.Word:
			return wordSelection(model, anchorRange, position, wordPattern);
		case MouseSelectionKind.WholeLine:
			return wholeLineSelection(model, anchor.lineNumber, position.lineNumber);
		case MouseSelectionKind.ExtendToWord:
			return extendSelectionToWord(model, anchor, position, wordPattern);
		case MouseSelectionKind.ExtendToLine:
			return extendSelectionToLine(model, anchor, position.lineNumber);
	}
}

function wordSelection(model: TextModel, anchorRange: Range, position: Position, wordPattern: RegExp | undefined): Selection {
	const activeRange = WordOperations.getWordSelectionRange(model, position, wordPattern);
	return Position.compare(activeRange.getStartPosition(), anchorRange.getStartPosition()) < 0
		? Selection.fromPositions(anchorRange.getEndPosition(), activeRange.getStartPosition())
		: Selection.fromPositions(anchorRange.getStartPosition(), activeRange.getEndPosition());
}

function extendSelectionToWord(model: TextModel, anchor: Position, position: Position, wordPattern: RegExp | undefined): Selection {
	const range = WordOperations.getWordSelectionRange(model, position, wordPattern);
	const active = Position.compare(range.getStartPosition(), anchor) < 0 ? range.getStartPosition() : range.getEndPosition();
	return Selection.fromPositions(anchor, active);
}

function wholeLineSelection(model: TextModel, anchorLineNumber: number, activeLineNumber: number): Selection {
	return activeLineNumber >= anchorLineNumber
		? Selection.fromPositions(lineStart(anchorLineNumber), lineEndExclusive(model, activeLineNumber))
		: Selection.fromPositions(lineEndExclusive(model, anchorLineNumber), lineStart(activeLineNumber));
}

function extendSelectionToLine(model: TextModel, anchor: Position, activeLineNumber: number): Selection {
	const active = activeLineNumber < anchor.lineNumber ? lineStart(activeLineNumber) : lineEndExclusive(model, activeLineNumber);
	return Selection.fromPositions(anchor, active);
}

function lineStart(lineNumber: number): Position {
	return new Position(lineNumber, 1);
}

function lineEndExclusive(model: TextModel, lineNumber: number): Position {
	return lineNumber < model.lineCount
		? new Position(lineNumber + 1, 1)
		: new Position(lineNumber, model.getLineContent(lineNumber).length + 1);
}

function createEditContext(container: HTMLElement, options: EditContextOptions): AbstractEditContext {
	const ownerWindow = container.ownerDocument.defaultView as NativeEditContextWindow | null;
	return typeof ownerWindow?.EditContext === 'function'
		? new NativeEditContext(container, options)
		: new TextAreaEditContext(container, options);
}

let viewId = 1;

function nextViewId(): string { return `zeta-editor-view-${viewId++}`; }

function validateOwnerId(value: string): string {
	if (typeof value !== 'string' || value.trim().length === 0) throw new TypeError('Editor view ownerId must be a non-empty string');
	return value;
}

function validateAccessibilityPageSize(value: number | undefined): void {
	if (value !== undefined && (!Number.isSafeInteger(value) || value < 1 || value > 10_000)) {
		throw new RangeError('Editor accessibility page size must be a safe integer between 1 and 10000');
	}
}

/** Browser input adapter for DOM-free language editing commands. */
export class LanguageEditingAdapter extends Disposable implements EditorLanguageEditingAdapter {
	private readonly lexicalContext: LanguageLexicalContextSource;

	constructor(readonly textModel: TextModel, private readonly selections: CursorsController, private readonly languageId: string, private readonly configurations: ILanguageConfigurationService, lexicalContext: LanguageLexicalContextSource | undefined = undefined, private readonly indentation: EditorIndentationOptions | undefined = undefined) {
		super();
		assertLanguageId(languageId);
		if (!configurations || typeof configurations.getLanguageConfiguration !== "function") throw new TypeError("Stanza text input language requires a configuration source");
		resolveEditorIndentationOptions(indentation);
		if (lexicalContext && (lexicalContext.textModel !== textModel || lexicalContext.languageId !== languageId)) throw new TypeError("Stanza text input lexical context must match its model and language");
		this.lexicalContext = lexicalContext ?? this._register(new LanguageLexicalContextIndex(textModel, languageId, configurations));
	}

	createTypeCommand(selections: readonly Selection[], text: string): EditorLanguageTypeCommand | undefined {
		const result = TypeOperations.typeWithInterceptors(
			this.textModel,
			selections,
			text,
			this.configurationAt(selections[0]!.getPosition()),
			this.selections.getAutoClosedCharacters(),
			this.lexicalContext,
		);
		if (!result) return undefined;
		return Object.freeze({
			command: result.command,
			insertedText: result.insertedText,
			afterExecute: (change: TextModelChange) => {
				if (result.autoClosedCharacters.length === 0) return;
				this.selections.recordAutoClosedCharacters(
					result.autoClosedCharacters.map(range => Range.fromPositions(this.textModel.positionAt(range.startOffset), this.textModel.positionAt(range.endOffset))),
					result.autoClosedEnclosing.map(range => Range.fromPositions(this.textModel.positionAt(range.startOffset), this.textModel.positionAt(range.endOffset))),
					change.version,
				);
			},
		});
	}

	createEnterCommand(selections: readonly Selection[]): EditorEditCommand {
		return TypeOperations.enter(this.textModel, selections, this.configurationAt(selections[0]!.getPosition()), this.indentation, this.lexicalContext);
	}

	createBackspaceCommand(selections: readonly Selection[]): EditorEditCommand {
		return DeleteOperations.deleteLeft(
			this.textModel,
			selections,
			this.configurationAt(selections[0]!.getPosition()),
			this.selections.getAutoClosedCharacters(),
		);
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
		userInputEvents: ViewUserInputEvents,
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
			: MoveOperations.navigate(
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
		this.viewport.revealPosition(result.selections[0]!.getPosition());
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
