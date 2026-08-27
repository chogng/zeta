import { addDisposableListener } from '../../base/browser/dom.js';
import { type Event, Emitter } from '../../base/common/event.js';
import { Disposable, toDisposable } from '../../base/common/lifecycle.js';
import {
	constObservable,
	derived,
	isObservable,
	transaction,
	type IObservable,
	type IReader,
	type ISettableObservable,
	type ITransaction,
} from '../../base/common/observable.js';
import { type IPosition, TextPosition } from '../common/core/position.js';
import { Point } from '../common/core/2d/point.js';
import { LineRange } from '../common/core/ranges/lineRange.js';
import { TextSelection, type TextSelectionSet } from '../common/core/selection.js';
import { type EditorSelectionController } from '../common/cursor/editorSelectionController.js';
import { type TextModel } from '../common/model/textModel.js';
import { type EditorViewportLayout } from '../common/viewLayout/viewLayout.js';
import { type IClipboardPasteEvent } from './controller/editContext/clipboardUtils.js';
import { CodeEditorWidget } from './widget/codeEditor/codeEditorWidget.js';

/** Returns the observable facade for one canonical Stanza code editor widget. */
export function observableCodeEditor(editor: CodeEditorWidget): ObservableCodeEditor {
	return ObservableCodeEditor.get(editor);
}

const observableEditorCache = new WeakMap<CodeEditorWidget, ObservableCodeEditor>();

/**
 * Observable state exposed by a {@link CodeEditorWidget}.
 *
 * The facade observes the widget's existing model, selection controller,
 * EditContext, and viewport. It does not create a second text model, scroll
 * owner, or DOM projection.
 */
export class ObservableCodeEditor extends Disposable {
	private readonly modelState: ObservableState<TextModel>;
	private readonly versionState: ObservableState<number>;
	private readonly selectionsState: ObservableState<TextSelectionSet>;
	private readonly focusState: ObservableState<boolean>;
	private readonly textFocusState: ObservableState<boolean>;
	private readonly compositionState: ObservableState<boolean>;
	private readonly layoutState: ObservableState<EditorViewportLayout>;
	private readonly typeChannel: ObservableChannel<string>;
	private readonly pasteChannel: ObservableChannel<IClipboardPasteEvent | undefined>;
	private currentTransaction: ITransaction | undefined;

	public readonly editor: CodeEditorWidget;
	public readonly model: IObservable<TextModel>;
	public readonly isReadonly: IObservable<boolean>;
	public readonly versionId: IObservable<number>;
	public readonly selections: IObservable<TextSelectionSet>;
	public readonly positions: IObservable<readonly TextPosition[]>;
	public readonly isFocused: IObservable<boolean>;
	public readonly isTextFocused: IObservable<boolean>;
	public readonly inComposition: IObservable<boolean>;
	public readonly value: ISettableObservable<string>;
	public readonly valueIsEmpty: IObservable<boolean>;
	public readonly cursorSelection: IObservable<TextSelection>;
	public readonly cursorPosition: IObservable<TextPosition>;
	/** The primary cursor's zero-based line index. */
	public readonly cursorLineIndex: IObservable<number>;
	/** The primary cursor's one-based line number. */
	public readonly cursorLineNumber: IObservable<number>;
	public readonly onDidType: IObservable<string>;
	/** The latest normalized paste event; Zeta exposes it at the EditContext boundary. */
	public readonly onDidPaste: IObservable<IClipboardPasteEvent | undefined>;
	public readonly layoutInfo: IObservable<EditorViewportLayout>;
	public readonly layoutInfoContentLeft: IObservable<number>;
	public readonly layoutInfoDecorationsLeft: IObservable<number>;
	public readonly layoutInfoWidth: IObservable<number>;
	public readonly layoutInfoHeight: IObservable<number>;
	public readonly scrollTop: IObservable<number>;
	public readonly scrollLeft: IObservable<number>;
	public readonly contentWidth: IObservable<number>;
	public readonly contentHeight: IObservable<number>;
	public readonly domNode: IObservable<HTMLDivElement>;

	public static get(editor: CodeEditorWidget): ObservableCodeEditor {
		const existing = observableEditorCache.get(editor);
		if (existing) return existing;
		const result = new ObservableCodeEditor(editor);
		observableEditorCache.set(editor, result);
		return result;
	}

	private constructor(editor: CodeEditorWidget) {
		super();
		this.editor = editor;
		this._register(toDisposable(() => {
			if (observableEditorCache.get(editor) === this) observableEditorCache.delete(editor);
		}));

		const selectionController = editor.view.selectionController;
		const model = editor.viewport.textModel;
		this.modelState = this._register(new ObservableState(model));
		this.versionState = this._register(new ObservableState(model.version));
		this.selectionsState = this._register(new ObservableState(selectionController.selections));
		this.focusState = this._register(new ObservableState(readFocus(editor.element)));
		this.textFocusState = this._register(new ObservableState(readFocus(editor.view.editContext.element)));
		this.compositionState = this._register(new ObservableState(editor.view.compositionController.composing));
		this.layoutState = this._register(new ObservableState(editor.viewport.currentLayout));
		this.typeChannel = this._register(new ObservableChannel(''));
		this.pasteChannel = this._register(new ObservableChannel<IClipboardPasteEvent | undefined>(undefined));

		this.model = this.modelState;
		this.isReadonly = constObservable(selectionController.readOnly);
		this.versionId = this.versionState;
		this.selections = this.selectionsState;
		this.positions = derived(reader => Object.freeze(
			this.selections.read(reader).selections.map(selection => selection.getSelectionStart()),
		));
		this.isFocused = this.focusState;
		this.isTextFocused = this.textFocusState;
		this.inComposition = this.compositionState;

		const valueSource = derived(reader => {
			this.versionId.read(reader);
			return this.model.read(reader).getText();
		});
		this.value = this._register(new SettableDerivedObservable(valueSource, (value, suppliedTransaction) => {
			const update = (): void => {
				const currentModel = this.modelState.get();
				if (currentModel.getText() !== value) currentModel.reset(value);
			};
			if (suppliedTransaction && suppliedTransaction === this.currentTransaction) update();
			else this.runInTransaction(() => update());
		}));
		this.valueIsEmpty = derived(reader => {
			this.versionId.read(reader);
			return this.model.read(reader).length === 0;
		});
		this.cursorSelection = derived(reader => this.selections.read(reader).primary);
		this.cursorPosition = derived(reader => this.cursorSelection.read(reader).getPosition());
		this.cursorLineIndex = derived(reader => this.cursorPosition.read(reader).lineIndex);
		this.cursorLineNumber = derived(reader => this.cursorLineIndex.read(reader) + 1);

		this.onDidType = this.typeChannel;
		this.onDidPaste = this.pasteChannel;
		this.layoutInfo = this.layoutState;
		this.layoutInfoContentLeft = derived(reader => {
			this.layoutInfo.read(reader);
			this.value.read(reader);
			return editor.viewport.getPositionContentCoordinates(TextPosition.at(0, 0)).left;
		});
		this.layoutInfoDecorationsLeft = this.layoutInfoContentLeft;
		this.layoutInfoWidth = this.layoutInfo.map(layout => layout.viewportSize.width);
		this.layoutInfoHeight = this.layoutInfo.map(layout => layout.viewportSize.height);
		this.scrollTop = this.layoutInfo.map(layout => layout.scrollPosition.top);
		this.scrollLeft = this.layoutInfo.map(layout => layout.scrollPosition.left);
		this.contentWidth = this.layoutInfo.map(layout => layout.contentSize.width);
		this.contentHeight = this.layoutInfo.map(layout => layout.contentSize.height);
		this.domNode = derived(reader => {
			this.model.read(reader);
			return editor.element;
		});

		this._register(model.onDidChange(() => this.runInTransaction(transaction => this.synchronizeState(transaction))));
		this._register(selectionController.onDidChange(() => this.runInTransaction(transaction => this.synchronizeState(transaction))));
		this._register(editor.viewport.onDidChangeLayout(change => this.runInTransaction(transaction => {
			this.layoutState.set(change.layout, transaction);
			this.synchronizeState(transaction);
		})));
		this._register(editor.view.onDidEdit(event => this.runInTransaction(transaction => {
			this.synchronizeState(transaction);
			if (event.insertedText !== undefined) this.typeChannel.emit(event.insertedText, transaction);
		})));
		this._register(editor.view.editContext.onWillPaste(event => this.runInTransaction(transaction => {
			this.pasteChannel.emit(event, transaction);
		})));
		this._register(editor.view.compositionController.onDidChange(composing => this.runInTransaction(transaction => {
			this.compositionState.set(composing, transaction);
		})));
		this._register(editor.view.editContext.onDidFocus(() => this.runInTransaction(transaction => {
			this.focusState.set(true, transaction);
			this.textFocusState.set(true, transaction);
		})));
		this._register(editor.view.editContext.onDidBlur(() => this.runInTransaction(transaction => {
			this.focusState.set(readFocus(editor.element), transaction);
			this.textFocusState.set(false, transaction);
		})));
		this._register(addDisposableListener(editor.element, 'focusin', () => this.refreshFocusState()));
		this._register(addDisposableListener(editor.element, 'focusout', () => this.refreshFocusState()));
	}

	/** Batches state notifications caused by synchronous editor work. */
	public transaction<T>(callback: (transaction: ITransaction) => T): T {
		return this.runInTransaction(callback);
	}

	/** Re-reads model, selection, composition, and layout state immediately. */
	public forceUpdate(): void;
	public forceUpdate<T>(callback: (transaction: ITransaction) => T): T;
	public forceUpdate<T>(callback?: (transaction: ITransaction) => T): T | undefined {
		return this.runInTransaction(transaction => {
			this.synchronizeState(transaction, true);
			return callback ? callback(transaction) : undefined;
		});
	}

	/** Returns the model-relative x offset of a position from the text origin. */
	public getLeftOfPosition(position: IPosition, reader: IReader | undefined = undefined): number {
		this.layoutInfo.read(reader);
		this.value.read(reader);
		const textOrigin = this.editor.viewport.getPositionContentCoordinates(TextPosition.at(0, 0)).left;
		return this.editor.viewport.getPositionContentCoordinates(TextPosition.lift(position)).left - textOrigin;
	}

	/** Observes a position in viewport coordinates, including scroll changes. */
	public observePosition(position: IObservable<IPosition | null>): IObservable<Point | null> {
		return derived(reader => {
			const currentPosition = position.read(reader);
			if (currentPosition === null) return null;
			const layout = this.layoutInfo.read(reader);
			const model = this.model.read(reader);
			const liftedPosition = TextPosition.lift(currentPosition);
			model.offsetAt(liftedPosition);
			const coordinates = this.editor.viewport.getPositionContentCoordinates(liftedPosition);
			return new Point(
				coordinates.left - layout.scrollPosition.left,
				coordinates.top - layout.scrollPosition.top,
			);
		});
	}

	public observeLineHeightForPosition(position: IObservable<IPosition | null>): IObservable<number | null>;
	public observeLineHeightForPosition(position: IPosition): IObservable<number>;
	public observeLineHeightForPosition(position: IObservable<IPosition | null> | IPosition): IObservable<number | null> {
		return derived(reader => {
			const currentPosition = readValue(position, reader);
			if (currentPosition === null) return null;
			const model = this.model.read(reader);
			model.offsetAt(TextPosition.lift(currentPosition));
			return this.layoutInfo.read(reader).lineHeight;
		});
	}

	public observeLineHeightForLine(lineIndex: IObservable<number | null>): IObservable<number | null>;
	public observeLineHeightForLine(lineIndex: number): IObservable<number>;
	public observeLineHeightForLine(lineIndex: IObservable<number | null> | number): IObservable<number | null> {
		return derived(reader => {
			const currentLineIndex = readValue(lineIndex, reader);
			if (currentLineIndex === null) return null;
			return this.readLineHeight(currentLineIndex, reader);
		});
	}

	public observeLineHeightsForLineRange(lineRange: IObservable<LineRange> | LineRange): IObservable<readonly number[]> {
		return derived(reader => {
			const currentLineRange = readValue(lineRange, reader);
			return currentLineRange.mapToLineArray(lineIndex => this.readLineHeight(lineIndex, reader));
		});
	}

	public getWidthOfLine(lineIndex: number, reader: IReader | undefined = undefined): number {
		this.layoutInfo.read(reader);
		this.value.read(reader);
		const model = this.model.read(reader);
		validateLineIndex(model, lineIndex);
		return this.editor.viewport.measureTextWidth(model.getLineContent(lineIndex));
	}

	public observeTopForLineNumber(lineIndex: number): IObservable<number> {
		return derived(reader => {
			this.layoutInfo.read(reader);
			const model = this.model.read(reader);
			return this.readLineTop(model, lineIndex);
		});
	}

	public observeBottomForLineNumber(lineIndex: number): IObservable<number> {
		return derived(reader => {
			this.layoutInfo.read(reader);
			const model = this.model.read(reader);
			return this.readLineBottom(model, lineIndex);
		});
	}

	private runInTransaction<T>(callback: (transaction: ITransaction) => T): T {
		if (this.currentTransaction) return callback(this.currentTransaction);
		return transaction(activeTransaction => {
			this.currentTransaction = activeTransaction;
			try {
				return callback(activeTransaction);
			} finally {
				this.currentTransaction = undefined;
			}
		});
	}

	private synchronizeState(transaction: ITransaction, force = false): void {
		const model = this.editor.viewport.textModel;
		const selectionController = this.editor.view.selectionController;
		this.modelState.update(model, transaction, force);
		this.versionState.update(model.version, transaction, force);
		this.selectionsState.update(selectionController.selections, transaction, force);
		this.layoutState.update(this.editor.viewport.currentLayout, transaction, force);
		this.focusState.update(readFocus(this.editor.element), transaction, force);
		this.textFocusState.update(readFocus(this.editor.view.editContext.element), transaction, force);
		this.compositionState.update(this.editor.view.compositionController.composing, transaction, force);
	}

	private refreshFocusState(): void {
		this.runInTransaction(transaction => {
			this.focusState.set(readFocus(this.editor.element), transaction);
			this.textFocusState.set(readFocus(this.editor.view.editContext.element), transaction);
		});
	}

	private readLineHeight(lineIndex: number, reader: IReader): number {
		const model = this.model.read(reader);
		validateLineIndex(model, lineIndex);
		return this.layoutInfo.read(reader).lineHeight;
	}

	private readLineTop(model: TextModel, lineIndex: number): number {
		validateLineIndexOrEnd(model, lineIndex);
		if (lineIndex === model.lineCount) return this.readLineBottom(model, model.lineCount - 1);
		return this.editor.viewport.getPositionContentCoordinates(TextPosition.at(lineIndex, 0)).top;
	}

	private readLineBottom(model: TextModel, lineIndex: number): number {
		validateLineIndexOrEnd(model, lineIndex);
		if (lineIndex === model.lineCount) return this.readLineBottom(model, model.lineCount - 1);
		const position = TextPosition.at(lineIndex, model.getLineLength(lineIndex));
		const coordinates = this.editor.viewport.getPositionContentCoordinates(position);
		return coordinates.top + coordinates.height;
	}
}

class ObservableState<T> extends Disposable implements ISettableObservable<T> {
	private readonly emitter: Emitter<T>;
	private value: T;

	public readonly onDidChange: Event<T>;

	public constructor(initialValue: T) {
		super();
		this.value = initialValue;
		this.emitter = this._register(new Emitter<T>());
		this.onDidChange = this.emitter.event;
	}

	public get(): T {
		return this.value;
	}

	public read(reader: IReader | undefined): T {
		return reader ? reader.readObservable(this) : this.value;
	}

	public map<TMapped>(mapValue: (value: T, reader: IReader) => TMapped): IObservable<TMapped> {
		return derived(reader => mapValue(this.read(reader), reader));
	}

	public set(value: T, activeTransaction?: ITransaction): void {
		this.update(value, activeTransaction, false);
	}

	public update(value: T, activeTransaction: ITransaction | undefined, force: boolean): void {
		if (!force && Object.is(this.value, value)) return;
		this.value = value;
		this.publish(activeTransaction);
	}

	private publish(activeTransaction: ITransaction | undefined): void {
		if (activeTransaction) activeTransaction.enqueue(this, () => this.emitter.fire(this.value));
		else this.emitter.fire(this.value);
	}
}

class ObservableChannel<T> extends Disposable implements IObservable<T> {
	private readonly emitter: Emitter<T>;
	private value: T;

	public readonly onDidChange: Event<T>;

	public constructor(initialValue: T) {
		super();
		this.value = initialValue;
		this.emitter = this._register(new Emitter<T>());
		this.onDidChange = this.emitter.event;
	}

	public get(): T {
		return this.value;
	}

	public read(reader: IReader | undefined): T {
		return reader ? reader.readObservable(this) : this.value;
	}

	public map<TMapped>(mapValue: (value: T, reader: IReader) => TMapped): IObservable<TMapped> {
		return derived(reader => mapValue(this.read(reader), reader));
	}

	public emit(value: T, activeTransaction: ITransaction | undefined): void {
		this.value = value;
		if (activeTransaction) activeTransaction.enqueue(this, () => this.emitter.fire(this.value));
		else this.emitter.fire(value);
	}
}

class SettableDerivedObservable<T> extends Disposable implements ISettableObservable<T> {
	public constructor(
		private readonly source: IObservable<T>,
		private readonly setter: (value: T, transaction: ITransaction | undefined) => void,
	) {
		super();
	}

	public get(): T {
		return this.source.get();
	}

	public read(reader: IReader | undefined): T {
		return this.source.read(reader);
	}

	public onDidChange: Event<T> = listener => this.source.onDidChange(listener);

	public map<TMapped>(mapValue: (value: T, reader: IReader) => TMapped): IObservable<TMapped> {
		return derived(reader => mapValue(this.read(reader), reader));
	}

	public set(value: T, transaction?: ITransaction): void {
		this.setter(value, transaction);
	}
}

function readValue<T>(value: T | IObservable<T>, reader: IReader): T {
	return isObservable(value)
		? (value as IObservable<T>).read(reader)
		: value;
}

function readFocus(element: HTMLElement): boolean {
	const activeElement = element.ownerDocument.activeElement;
	return activeElement === element || Boolean(activeElement && element.contains(activeElement));
}

function validateLineIndex(model: TextModel, lineIndex: number): void {
	if (!Number.isSafeInteger(lineIndex) || lineIndex < 0 || lineIndex >= model.lineCount) {
		throw new RangeError(`Line index must be between 0 and ${model.lineCount - 1}`);
	}
}

function validateLineIndexOrEnd(model: TextModel, lineIndex: number): void {
	if (!Number.isSafeInteger(lineIndex) || lineIndex < 0 || lineIndex > model.lineCount) {
		throw new RangeError(`Line index must be between 0 and ${model.lineCount}`);
	}
}
