import { type IKeyboardEvent } from '../../../base/browser/keyboardEvent.js';
import { type IMouseWheelEvent } from '../../../base/browser/mouseEvent.js';
import { addDisposableListener, getClientArea } from '../../../base/browser/dom.js';
import { Emitter, type Event } from '../../../base/common/event.js';
import { Disposable, toDisposable, type IDisposable } from '../../../base/common/lifecycle.js';
import { isLinux, operatingSystem, OperatingSystem } from '../../../base/common/platform.js';
import { type EditorEditCommand } from '../../common/commands/editorEditCommand.js';
import { ReplaceCommand } from '../../common/commands/replaceCommand.js';
import { EditorLineWrapping, EditorOption } from '../../common/config/editorOptions.js';
import { ColumnSelection } from '../../common/cursor/cursorColumnSelection.js';
import { CursorMove, CursorMoveCommands } from '../../common/cursor/cursorMoveCommands.js';
import { DeleteOperations } from '../../common/cursor/cursorDeleteOperations.js';
import { WordOperations } from '../../common/cursor/cursorWordOperations.js';
import { type CursorsController } from '../../common/cursor/cursor.js';
import { CursorChangeReason } from '../../common/cursorEvents.js';
import { CursorState, EditOperationType, type SingleCursorState } from '../../common/cursorCommon.js';
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
import { EditorCursorNavigationCommand, EditorCursorNavigationMode, navigateStanzaVisualCursors } from '../../common/viewModel/visualCursorNavigation.js';
import { type IViewModel } from '../../common/viewModel.js';
import { ViewEventHandler } from '../../common/viewEventHandler.js';
import { type View } from '../view.js';
import { NavigationCommandRevealType } from '../coreCommands.js';
import { type AbstractEditContext, type CompositionController, type EditContextTextUpdate } from '../controller/editContext/editContext.js';
import { TextAreaEditContext } from '../controller/editContext/textArea/textAreaEditContext.js';
import { ViewUserInputEvents } from './viewUserInputEvents.js';
import { type IAccessibilityService } from '../../../platform/accessibility/common/accessibility.js';
import { type IEditorAriaOptions, type IEditorMouseEvent, type IPartialEditorMouseEvent } from '../editorBrowser.js';
import { type BracketColorizationSource, type SemanticTokenSource } from '../viewParts/viewLines/viewLine.js';
import { type ILogService } from '../../../platform/log/common/log.js';
import { type ICommand } from '../../common/editorCommon.js';

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
	readonly logService?: ILogService;
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
		options: ViewControllerOptions,
		createEditContext: (viewController: ViewController) => AbstractEditContext,
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
			this.editContext = createEditContext(this);
			this.element = this.editContext.domNode.domNode;
			this.textArea = this.editContext instanceof TextAreaEditContext ? this.editContext.getTextAreaDomNode() : undefined;
			this.compositionController = this.editContext.compositionController;
			this.onWillBeforeInput = this.editContext.onWillBeforeInput;
			this.onWillTextUpdate = this.editContext.onWillTextUpdate;
			this.onWillKeydown = this.editContext.onWillKeydown;
			this._register(this.onDidChangeOvertype(overtyping => {
				viewport.element.classList.toggle('overtype', overtyping);
				viewport.setOvertype(overtyping);
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
			toggleCandidateIndex: baseSelections ? findPointerToggleCandidate(baseSelections, initialSelection) : undefined,
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
			? combinePointerSelection(state.baseSelections, selection, state.toggleCandidateIndex)
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
		return this.executeDelete('left', this.selectionController.selections, inputType);
	}

	public deleteForward(inputType = 'deleteContentForward'): TextModelChange | undefined {
		return this.executeDelete('right', this.selectionController.selections, inputType);
	}

	public deleteWordBackward(inputType = 'deleteWordBackward'): TextModelChange | undefined {
		return this.execute(WordOperations.deleteWordLeft(this.viewport.textModel, this.selectionController.selections, this.currentWordPattern), inputType);
	}

	public deleteWordForward(inputType = 'deleteWordForward'): TextModelChange | undefined {
		return this.execute(WordOperations.deleteWordRight(this.viewport.textModel, this.selectionController.selections, this.currentWordPattern), inputType);
	}

	public deleteSoftLineBackward(inputType = 'deleteSoftLineBackward'): TextModelChange | undefined {
		return this.executeCommands(createDeleteToLineBoundaryCommands(this.viewport.textModel, this.selectionController.selections, 'start'), inputType, EditOperationType.Other, true, true);
	}

	public deleteSoftLineForward(inputType = 'deleteSoftLineForward'): TextModelChange | undefined {
		return this.executeCommands(createDeleteToLineBoundaryCommands(this.viewport.textModel, this.selectionController.selections, 'end'), inputType, EditOperationType.Other, true, true);
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
		if (inputType === 'deleteContentForward') return this.executeDelete('right', selections, inputType);
		if (inputType === 'deleteContentBackward') {
			return this.executeDelete('left', selections, inputType);
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

	private executeDelete(direction: 'left' | 'right', selections: readonly Selection[], inputType: string): TextModelChange | undefined {
		const previousType = this.selectionController.getPrevEditOperationType();
		const operation = direction === 'left'
			? DeleteOperations.deleteLeft(previousType, this.viewport.cursorConfig, this.viewport.textModel, [...selections], [...this.selectionController.getAutoClosedCharacters()])
			: DeleteOperations.deleteRight(previousType, this.viewport.cursorConfig, this.viewport.textModel, [...selections]);
		return this.executeCommands(
			operation[1],
			inputType,
			direction === 'left' ? EditOperationType.DeletingLeft : EditOperationType.DeletingRight,
			operation[0],
			false,
		);
	}

	private executeCommands(commands: readonly (ICommand | null)[], inputType: string, type: EditOperationType, pushBefore: boolean, pushAfter: boolean): TextModelChange | undefined {
		if (pushBefore) this.selectionController.pushUndoStop();
		let change: TextModelChange | undefined;
		const capture = this.viewport.textModel.onDidChangeContent(event => { change = event; });
		try {
			this.selectionController.executeCommands(commands, inputType);
		} finally {
			capture.dispose();
		}
		this.selectionController.setPrevEditOperationType(type);
		if (pushAfter) this.selectionController.pushUndoStop();
		this.revealPrimary();
		if (change) this.didEditEmitter.fire(Object.freeze({ inputType, insertedText: undefined, change }));
		return change;
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

}

function findPointerToggleCandidate(selections: readonly Selection[], selection: Selection): number | undefined {
	const index = selections.findIndex(candidate => selectionsHaveSameRange(candidate, selection));
	return index < 0 ? undefined : index;
}

function combinePointerSelection(selections: readonly Selection[], active: Selection, toggleIndex: number | undefined): readonly Selection[] {
	if (toggleIndex !== undefined && (!Number.isSafeInteger(toggleIndex) || toggleIndex < 0 || toggleIndex >= selections.length)) {
		throw new RangeError('Pointer toggle candidate is outside the selection set');
	}
	if (toggleIndex !== undefined && selectionsHaveSameRange(selections[toggleIndex]!, active)) {
		return selections.length === 1 ? selections : Object.freeze(selections.filter((_, index) => index !== toggleIndex));
	}
	const retained = toggleIndex === undefined ? [...selections] : selections.filter((_, index) => index !== toggleIndex);
	const duplicateIndex = retained.findIndex(selection => selectionsHaveSameRange(selection, active));
	if (duplicateIndex >= 0) {
		return Object.freeze([retained[duplicateIndex]!, ...retained.slice(0, duplicateIndex), ...retained.slice(duplicateIndex + 1)]);
	}
	return Object.freeze([active, ...retained.filter(selection => !selectionRangesOverlap(selection, active))]);
}

function selectionsHaveSameRange(left: Selection, right: Selection): boolean {
	return left.getStartPosition().equals(right.getStartPosition()) && left.getEndPosition().equals(right.getEndPosition());
}

function selectionRangesOverlap(left: Selection, right: Selection): boolean {
	if (left.isEmpty()) return pointOverlapsSelection(left.getPosition(), right);
	if (right.isEmpty()) return pointOverlapsSelection(right.getPosition(), left);
	return left.getStartPosition().isBefore(right.getEndPosition()) && right.getStartPosition().isBefore(left.getEndPosition());
}

function pointOverlapsSelection(point: Position, selection: Selection): boolean {
	return selection.isEmpty()
		? point.equals(selection.getPosition())
		: !point.isBefore(selection.getStartPosition()) && point.isBefore(selection.getEndPosition());
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

	private configurationAt(position: Position) {
		return this.configurations.getLanguageConfiguration(this.lexicalContext.getLanguageIdAt(position));
	}
}

function createDeleteToLineBoundaryCommands(model: TextModel, selections: readonly Selection[], boundary: 'start' | 'end'): Array<ICommand | null> {
	return selections.map(selection => {
		const position = selection.getPosition();
		const range = selection.isEmpty()
			? boundary === 'start'
				? Range.fromPositions(new Position(position.lineNumber, 1), position)
				: Range.fromPositions(position, new Position(position.lineNumber, model.getLineMaxColumn(position.lineNumber)))
			: selection;
		return range.isEmpty() ? null : new ReplaceCommand(range, '');
	});
}

export interface KeyboardNavigationControllerOptions {
	readonly operatingSystem?: OperatingSystem;
	/** Resolves the active language word matcher for word navigation. */
	readonly wordPattern?: () => RegExp | undefined;
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
	private modelNavigationStates: readonly SingleCursorState[] | undefined;
	private preferredVisualHorizontalOffsets: readonly number[] | undefined;
	private applyingNavigation = false;

	constructor(
		private readonly viewport: View,
		private readonly viewModel: IViewModel,
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
			this.wordPattern = options.wordPattern;
		} catch (error) {
			this.dispose();
			throw error;
		}
		if (viewport.textModel !== viewModel.model) {
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
		const cursorListener = this._register(new class extends ViewEventHandler {
			constructor(private readonly reset: () => void) { super(); }
			override onCursorStateChanged(): boolean {
				this.reset();
				return false;
			}
		}(() => {
			if (this.applyingNavigation) return;
			this.modelNavigationStates = undefined;
			this.preferredVisualHorizontalOffsets = undefined;
		}));
		viewModel.addViewEventHandler(cursorListener);
		this._register(toDisposable(() => viewModel.removeViewEventHandler(cursorListener)));
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
		const visualResult = this.viewport.lineWrapping === EditorLineWrapping.On &&
			visualCommand !== undefined
			? navigateStanzaVisualCursors(
				this.viewport.textModel,
				this.viewport.getVisualLineProjection(),
				this.viewModel.getCursorStates().map(state => state.modelState.selection),
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
			: undefined;
		const currentStates = this.viewModel.getCursorStates();
		const sourceStates = this.modelNavigationStates?.length === currentStates.length
			&& this.modelNavigationStates.every((state, index) => state.selection.equalsSelection(currentStates[index]!.modelState.selection))
			? currentStates.map((state, index) => new CursorState(this.modelNavigationStates![index]!, state.viewState))
			: currentStates;
		const movedStates = visualResult ? undefined : moveCursorStates(
			this.viewModel, sourceStates, navigation, pageLineCount, this.wordPattern?.(),
		);
		const modelStates = visualResult
			? visualResult.selections.map(selection => CursorState.fromModelSelection(selection))
			: movedStates!;
		this.applyingNavigation = true;
		try {
			this.viewModel.setCursorStates('keyboard', CursorChangeReason.Explicit, modelStates);
		} finally {
			this.applyingNavigation = false;
		}
		this.modelNavigationStates = movedStates?.every(state => state.modelState !== null)
			? movedStates.map(state => state.modelState!)
			: undefined;
		this.preferredVisualHorizontalOffsets = visualResult?.preferredHorizontalOffsets;
		this.viewport.revealPosition(this.viewModel.getCursorStates()[0]!.modelState.position);
	}
}

function moveCursorStates(viewModel: IViewModel, cursors: CursorState[], navigation: KeyboardNavigationCommand, pageLineCount: number, wordPattern: RegExp | undefined) {
	const inSelectionMode = navigation.mode === EditorCursorNavigationMode.Extend;
	switch (navigation.command) {
		case EditorCursorNavigationCommand.CharacterLeft:
			return CursorMoveCommands.simpleMove(viewModel, cursors, CursorMove.Direction.Left, inSelectionMode, 1, CursorMove.Unit.Character)!;
		case EditorCursorNavigationCommand.CharacterRight:
			return CursorMoveCommands.simpleMove(viewModel, cursors, CursorMove.Direction.Right, inSelectionMode, 1, CursorMove.Unit.Character)!;
		case EditorCursorNavigationCommand.LineUp:
			return CursorMoveCommands.simpleMove(viewModel, cursors, CursorMove.Direction.Up, inSelectionMode, 1, CursorMove.Unit.Line)!;
		case EditorCursorNavigationCommand.LineDown:
			return CursorMoveCommands.simpleMove(viewModel, cursors, CursorMove.Direction.Down, inSelectionMode, 1, CursorMove.Unit.Line)!;
		case EditorCursorNavigationCommand.PageUp:
			return CursorMoveCommands.simpleMove(viewModel, cursors, CursorMove.Direction.Up, inSelectionMode, pageLineCount, CursorMove.Unit.Line)!;
		case EditorCursorNavigationCommand.PageDown:
			return CursorMoveCommands.simpleMove(viewModel, cursors, CursorMove.Direction.Down, inSelectionMode, pageLineCount, CursorMove.Unit.Line)!;
		case EditorCursorNavigationCommand.LineStart:
			return CursorMoveCommands.moveToBeginningOfLine(viewModel, cursors, inSelectionMode);
		case EditorCursorNavigationCommand.LineEnd:
			return CursorMoveCommands.moveToEndOfLine(viewModel, cursors, inSelectionMode, false);
		case EditorCursorNavigationCommand.DocumentStart:
			return CursorMoveCommands.moveToBeginningOfBuffer(viewModel, cursors, inSelectionMode);
		case EditorCursorNavigationCommand.DocumentEnd:
			return CursorMoveCommands.moveToEndOfBuffer(viewModel, cursors, inSelectionMode);
		case EditorCursorNavigationCommand.WordLeft:
			return cursors.map(cursor => CursorState.fromModelState(moveModelCursorByWord(viewModel.model, cursor.modelState, inSelectionMode, 'left', wordPattern)));
		case EditorCursorNavigationCommand.WordRight:
			return cursors.map(cursor => CursorState.fromModelState(moveModelCursorByWord(viewModel.model, cursor.modelState, inSelectionMode, 'right', wordPattern)));
	}
}

function moveModelCursorByWord(model: IViewModel['model'], cursor: SingleCursorState, inSelectionMode: boolean, direction: 'left' | 'right', wordPattern: RegExp | undefined): SingleCursorState {
	if (cursor.hasSelection() && !inSelectionMode) {
		const edge = direction === 'left' ? cursor.selection.getStartPosition() : cursor.selection.getEndPosition();
		return cursor.move(false, edge.lineNumber, edge.column, 0);
	}
	const target = wordNavigationPosition(model, cursor.position, direction, wordPattern);
	return cursor.move(inSelectionMode, target.lineNumber, target.column, 0);
}

function wordNavigationPosition(model: IViewModel['model'], position: Position, direction: 'left' | 'right', wordPattern: RegExp | undefined): Position {
	if (direction === 'left') {
		for (let lineNumber = position.lineNumber; lineNumber >= 1; lineNumber -= 1) {
			const limit = lineNumber === position.lineNumber ? position.column - 1 : Number.POSITIVE_INFINITY;
			const ranges = WordOperations.getTextWordRanges(model.getLineContent(lineNumber), wordPattern);
			for (let index = ranges.length - 1; index >= 0; index -= 1) {
				if (ranges[index]!.start < limit) return new Position(lineNumber, ranges[index]!.start + 1);
			}
		}
		return new Position(1, model.getLineMinColumn(1));
	}
	for (let lineNumber = position.lineNumber; lineNumber <= model.getLineCount(); lineNumber += 1) {
		const limit = lineNumber === position.lineNumber ? position.column - 1 : -1;
		for (const range of WordOperations.getTextWordRanges(model.getLineContent(lineNumber), wordPattern)) {
			if (range.start > limit) return new Position(lineNumber, range.start + 1);
		}
	}
	const lineNumber = model.getLineCount();
	return new Position(lineNumber, model.getLineMaxColumn(lineNumber));
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
