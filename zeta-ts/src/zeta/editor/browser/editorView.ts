import { addDisposableListener, getClientArea } from '../../base/browser/dom.js';
import { type Event } from '../../base/common/event.js';
import { Disposable, type IDisposable, toDisposable } from '../../base/common/lifecycle.js';
import { type IAccessibilityService } from '../../platform/accessibility/common/accessibility.js';
import { type CursorsController } from '../common/cursor/cursor.js';
import { type IDimension } from '../common/core/2d/dimension.js';
import { type TextModel } from '../common/model/textModel.js';
import { type AbstractEditContext, type CompositionController, type EditContextCharacterBounds, type EditContextOptions } from './controller/editContext/editContext.js';
import { createNativeEditContext, supportsNativeEditContext } from './controller/editContext/native/editContextFactory.js';
import { TextAreaEditContext } from './controller/editContext/textArea/textAreaEditContext.js';
import { ViewController, type EditorCommandContext, type EditorCommandTransformer, type EditorLanguageEditingAdapter, type EditorViewDidEditEvent, type EditorViewTextUpdateEvent } from './view/viewController.js';
import { type BracketColorizationSource, type SemanticTokenSource } from './viewParts/viewLines/viewLine.js';
import { type IEditorAriaOptions } from './editorBrowser.js';
import { ViewUserInputEvents } from './view/viewUserInputEvents.js';
import { View, type EditorViewportOptions } from './view.js';

export type EditorViewViewportOptions = Omit<EditorViewportOptions, 'container' | 'model' | 'lineHeight' | 'ariaLabel' | 'selectionController'>;

export type { EditorCommandContext, EditorCommandTransformer, EditorLanguageEditingAdapter, EditorLanguageTypeCommand, EditorViewDidEditEvent, EditorViewTextUpdateEvent } from './view/viewController.js';
export { ViewUserInputEvents } from './view/viewUserInputEvents.js';
export type { EventCallback, EditorViewMouseTargetKind, EditorViewMouseTarget, EditorViewMouseEvent, EditorViewPartialMouseEvent } from './view/viewUserInputEvents.js';

export interface EditorViewOptions {
	readonly container: HTMLElement;
	readonly model: TextModel;
	readonly selectionController: CursorsController;
	readonly lineHeight: number;
	readonly ariaLabel?: string;
	/** Stable identity used by host code that needs to address this view. */
	readonly ownerId?: string;
	readonly viewport?: EditorViewViewportOptions;
	readonly accessibilityService?: IAccessibilityService;
	readonly renderRichScreenReaderContent?: boolean;
	readonly accessibilityPageSize?: number;
	readonly semanticTokenSource?: SemanticTokenSource;
	readonly bracketColorizationSource?: BracketColorizationSource;
	/** Language-aware typing is supplied by an editor contribution, not the view itself. */
	readonly languageEditing?: EditorLanguageEditingAdapter;
	readonly wordPattern?: () => RegExp | undefined;
	/** Optional view-input bridge; the view creates one when omitted. */
	readonly userInputEvents?: ViewUserInputEvents;
}

/**
 * The browser view/input boundary for one line editor.
 *
 * This follows the VS Code split: the view selects and owns the concrete
 * EditContext adapters own browser input, while ViewController routes semantic
 * input into common commands.
 * View owns DOM projection and rendering; feature contributions own
 * policies such as completion.
 */
export class EditorView extends Disposable {
	readonly ownerId: string;
	readonly viewport: View;
	readonly selectionController: CursorsController;
	readonly editContext: AbstractEditContext;
	readonly element: HTMLElement;
	readonly textArea: HTMLTextAreaElement | undefined;
	readonly compositionController: CompositionController;
	readonly viewController: ViewController;
	readonly userInputEvents: ViewUserInputEvents;
	readonly onWillBeforeInput: Event<InputEvent>;
	readonly onWillTextUpdate: Event<EditorViewTextUpdateEvent>;
	readonly onWillKeydown: Event<KeyboardEvent>;
	readonly onDidEdit: Event<EditorViewDidEditEvent>;

	constructor(options: EditorViewOptions);
	/** Test and low-level integration overload for an already-created viewport. */
	constructor(viewport: View, selectionController: CursorsController, options?: Pick<EditorViewOptions, 'ariaLabel' | 'accessibilityService' | 'renderRichScreenReaderContent' | 'accessibilityPageSize' | 'semanticTokenSource' | 'bracketColorizationSource' | 'languageEditing' | 'wordPattern' | 'userInputEvents'>);
	constructor(
		optionsOrViewport: EditorViewOptions | View,
		legacySelectionController?: CursorsController,
		legacyOptions?: Pick<EditorViewOptions, 'ariaLabel' | 'accessibilityService' | 'renderRichScreenReaderContent' | 'accessibilityPageSize' | 'semanticTokenSource' | 'bracketColorizationSource' | 'languageEditing' | 'wordPattern' | 'userInputEvents'>,
	) {
		super();
		try {
			const existingViewport = optionsOrViewport instanceof View ? optionsOrViewport : undefined;
			const options = existingViewport ? undefined : optionsOrViewport as EditorViewOptions;
			const selectionController = existingViewport ? legacySelectionController : options!.selectionController;
			if (!selectionController) throw new TypeError('Editor view requires a selection controller');
			this.selectionController = selectionController;
			this.ownerId = options?.ownerId === undefined ? nextEditorViewId() : validateOwnerId(options.ownerId);
			this.viewport = existingViewport
				? existingViewport
				: this._register(new View({
					...options!.viewport,
					container: options!.container,
					model: options!.model,
					lineHeight: options!.lineHeight,
					ariaLabel: options!.ariaLabel,
					selectionController,
				}));
			const viewOptions = existingViewport ? legacyOptions ?? {} : options!;
			validateViewOptions(viewOptions);
			this.userInputEvents = viewOptions.userInputEvents ?? new ViewUserInputEvents();
			if (this.viewport.textModel !== selectionController.textModel) {
				throw new TypeError('Editor view and selection controller must share one text model');
			}
			if (viewOptions.languageEditing && viewOptions.languageEditing.textModel !== this.viewport.textModel) {
				throw new TypeError('Editor view language editing must share its text model');
			}
			if (viewOptions.semanticTokenSource && viewOptions.semanticTokenSource.textModel !== this.viewport.textModel) {
				throw new TypeError('Editor view semantic tokens must share its text model');
			}
			if (viewOptions.bracketColorizationSource && viewOptions.bracketColorizationSource.textModel !== this.viewport.textModel) {
				throw new TypeError('Editor view bracket colorization must share its text model');
			}

			const languageEditing = viewOptions.languageEditing;
			this.viewController = this._register(new ViewController(
				this.viewport,
				selectionController,
				{ languageEditing, wordPattern: viewOptions.wordPattern, userInputEvents: this.userInputEvents },
			));
			this.editContext = this._register(createEditContext(this.viewport.element, {
				ariaLabel: viewOptions.ariaLabel,
				readOnly: selectionController.readOnly,
				textDirection: this.viewport.editorTextDirection,
				ownerId: this.ownerId,
				characterBoundsProvider: modelOffset => this.characterBoundsAt(modelOffset),
				viewController: this.viewController,
				viewport: this.viewport,
				selectionController,
				accessibilityService: viewOptions.accessibilityService,
				renderRichScreenReaderContent: viewOptions.renderRichScreenReaderContent,
				accessibilityPageSize: viewOptions.accessibilityPageSize,
				semanticTokenSource: viewOptions.semanticTokenSource,
				bracketColorizationSource: viewOptions.bracketColorizationSource,
			}));
			this.element = this.editContext.domNode;
			this.textArea = this.editContext instanceof TextAreaEditContext
				? this.editContext.domNode
				: undefined;
			this.compositionController = this.editContext.compositionController;
			this.onWillBeforeInput = this.editContext.onWillBeforeInput;
			this.onWillTextUpdate = this.editContext.onWillTextUpdate;
			this.onWillKeydown = this.editContext.onWillKeydown;
			this.onDidEdit = this.viewController.onDidEdit;
			this._register(this.viewController.onDidChangeOvertype(overtyping => {
				this.viewport.element.classList.toggle('overtype', overtyping);
				this.viewport.setOvertype(overtyping);
			}));

			this._register(this.compositionController.onDidChange(composing => {
				if (!composing) this.synchronizeEditContext();
			}));
			this._register(toDisposable(() => {
				this.viewport.element.classList.remove('input-focused');
				this.viewport.element.classList.remove('overtype');
				this.viewport.setOvertype(false);
			}));
			this._register(addDisposableListener(this.viewport.element, 'focus', event => {
				if (event.target === this.viewport.element) this.focus();
			}));
			this._register(this.editContext.onDidFocus(() => {
				this.viewport.element.classList.add('input-focused');
			}));
			this._register(this.editContext.onDidBlur(() => {
				this.viewport.element.classList.remove('input-focused');
				this.editContext.clear();
			}));
			this._register(selectionController.onDidChange(() => this.synchronizeEditContext()));
			this._register(this.viewport.textModel.onDidChange(() => this.synchronizeEditContext()));
			this.synchronizeEditContext();
			this.editContext.connect();
		} catch (error) {
			this.dispose();
			throw error;
		}
	}

	get viewportElement(): HTMLDivElement {
		return this.viewport.element;
	}

	layout(dimension: IDimension = getClientArea(this.viewport.element)): void {
		this.viewport.layout({
			width: Math.max(0, dimension.width),
			height: Math.max(0, dimension.height),
		});
	}

	focus(): void {
		this.editContext.focus();
	}

	isFocused(): boolean {
		return this.editContext.isFocused();
	}

	refreshFocusState(): void {
		this.editContext.refreshFocusState();
	}

	setAriaOptions(options: IEditorAriaOptions): void {
		this.editContext.setAriaOptions(options);
	}

	writeScreenReaderContent(reason: string): void {
		this.editContext.writeScreenReaderContent(reason);
	}

	get overtyping(): boolean {
		return this.viewController.overtyping;
	}

	registerCommandTransformer(transformer: EditorCommandTransformer): IDisposable {
		return this.viewController.registerCommandTransformer(transformer);
	}

	/** Toggles this editor view's transient overtype mode. */
	toggleOvertype(): boolean {
		return this.viewController.toggleOvertype();
	}

	/** Reveals a model position for an input contribution after it commits an edit. */
	revealPosition(position: Parameters<View['revealPosition']>[0]): void {
		this.viewport.revealPosition(position);
	}

	clearInput(): void {
		this.editContext.clear();
	}

	private synchronizeEditContext(): void {
		const selection = this.selectionController.selections.primary;
		this.editContext.syncState({
			text: this.viewport.textModel.getText(),
			selectionStart: this.viewport.textModel.offsetAt(selection.getStartPosition()),
			selectionEnd: this.viewport.textModel.offsetAt(selection.getEndPosition()),
			position: selection.getPosition(),
		});
		this.editContext.updateBounds(
			this.viewport.getPositionContentCoordinates(selection.getPosition()),
		);
		this.editContext.writeScreenReaderContent('editor state changed');
	}

	private characterBoundsAt(modelOffset: number): EditContextCharacterBounds | undefined {
		const model = this.viewport.textModel;
		if (!Number.isSafeInteger(modelOffset) || modelOffset < 0 || modelOffset >= model.length) return undefined;
		const position = model.positionAt(modelOffset);
		const nextPosition = model.positionAt(Math.min(model.length, modelOffset + 1));
		const start = this.viewport.getPositionContentCoordinates(position);
		const end = this.viewport.getPositionContentCoordinates(nextPosition);
		const width = position.lineNumber === nextPosition.lineNumber
			? Math.max(1, Math.abs(end.left - start.left))
			: Math.max(1, this.viewport.measureTextWidth(' '));
		return Object.freeze({
			left: Math.min(start.left, end.left),
			top: start.top,
			width,
			height: start.height,
		});
	}
}

function createEditContext(
	container: HTMLElement,
	options: EditContextOptions,
): AbstractEditContext {
	if (supportsNativeEditContext(container)) {
		return createNativeEditContext(container, options);
	}
	return new TextAreaEditContext(container, options);
}

let nextViewId = 1;

function nextEditorViewId(): string {
	return `stanza-editor-view-${nextViewId++}`;
}

function validateOwnerId(ownerId: string): string {
	if (typeof ownerId !== 'string' || ownerId.trim().length === 0) {
		throw new TypeError('Editor view ownerId must be a non-empty string');
	}
	return ownerId;
}

function validateViewOptions(options: Pick<EditorViewOptions, 'accessibilityPageSize'>): void {
	validateAccessibilityPageSize(options.accessibilityPageSize);
}

function validateAccessibilityPageSize(pageSize: number | undefined): void {
	if (pageSize !== undefined && (!Number.isSafeInteger(pageSize) || pageSize < 1 || pageSize > 10_000)) {
		throw new RangeError('Editor accessibility page size must be a safe integer between 1 and 10000');
	}
}
