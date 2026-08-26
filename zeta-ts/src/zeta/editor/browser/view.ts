import { type Event } from '../../base/common/event.js';
import { getClientArea, type IDimension } from '../../base/browser/geometry.js';
import { addDisposableListener } from '../../base/browser/dom.js';
import { DisposableOwner, type IDisposable } from '../../base/common/lifecycle.js';
import { type IAccessibilityService } from '../../platform/accessibility/common/accessibility.js';
import { type EditorSelectionController } from '../common/cursor/editorSelectionController.js';
import { type TextModel } from '../common/model/textModel.js';
import { CompositionController } from './controller/compositionController.js';
import { createEditContext } from './controller/editContext/factory.js';
import { type EditContext, type EditContextCharacterBounds } from './controller/editContext/editContext.js';
import { NativeEditContext } from './controller/editContext/native/nativeEditContext.js';
import { ScreenReaderSupport } from './controller/editContext/native/screenReaderSupport.js';
import { TextAreaAccessibilityController } from './controller/editContext/textArea/textAreaAccessibilityController.js';
import { TextAreaEditContext } from './controller/editContext/textArea/textAreaEditContext.js';
import { ViewController, type EditorCommandContext, type EditorCommandTransformer, type EditorLanguageEditingAdapter, type EditorViewDidEditEvent, type EditorViewTextUpdateEvent } from './controller/viewController.js';
import { EditorViewport, type EditorViewportOptions } from './view/editorViewport.js';
import { type BracketColorizationSource, type SemanticTokenSource } from './viewparts/semanticTokens/semanticTokenPresentation.js';

export type EditorViewViewportOptions = Omit<EditorViewportOptions, 'container' | 'model' | 'lineHeight' | 'ariaLabel' | 'selectionController'>;

export type { EditorCommandContext, EditorCommandTransformer, EditorLanguageEditingAdapter, EditorLanguageTypeCommand, EditorViewDidEditEvent, EditorViewTextUpdateEvent } from './controller/viewController.js';

export interface EditorViewOptions {
	readonly container: HTMLElement;
	readonly model: TextModel;
	readonly selectionController: EditorSelectionController;
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
}

/**
 * The browser view/input boundary for one line editor.
 *
 * This follows the VS Code split: the view selects and owns the concrete
 * EditContext, while ViewController routes user input into common commands.
 * EditorViewport owns DOM projection and rendering; feature contributions own
 * policies such as completion.
 */
export class EditorView extends DisposableOwner {
	readonly ownerId: string;
	readonly viewport: EditorViewport;
	readonly selectionController: EditorSelectionController;
	readonly editContext: EditContext;
	/** Compatibility alias for integrations that call the browser surface input. */
	readonly input: EditContext;
	readonly element: HTMLElement;
	readonly textArea: HTMLTextAreaElement | undefined;
	readonly compositionController: CompositionController;
	readonly viewController: ViewController;
	readonly onWillBeforeInput: Event<InputEvent>;
	readonly onWillTextUpdate: Event<EditorViewTextUpdateEvent>;
	readonly onWillKeydown: Event<KeyboardEvent>;
	readonly onDidEdit: Event<EditorViewDidEditEvent>;

	constructor(options: EditorViewOptions);
	/** Test and low-level integration overload for an already-created viewport. */
	constructor(viewport: EditorViewport, selectionController: EditorSelectionController, options?: Pick<EditorViewOptions, 'ariaLabel' | 'accessibilityService' | 'renderRichScreenReaderContent' | 'accessibilityPageSize' | 'semanticTokenSource' | 'bracketColorizationSource' | 'languageEditing' | 'wordPattern'>);
	constructor(
		optionsOrViewport: EditorViewOptions | EditorViewport,
		legacySelectionController?: EditorSelectionController,
		legacyOptions?: Pick<EditorViewOptions, 'ariaLabel' | 'accessibilityService' | 'renderRichScreenReaderContent' | 'accessibilityPageSize' | 'semanticTokenSource' | 'bracketColorizationSource' | 'languageEditing' | 'wordPattern'>,
	) {
		super();
		try {
			const existingViewport = optionsOrViewport instanceof EditorViewport ? optionsOrViewport : undefined;
			const options = existingViewport ? undefined : optionsOrViewport as EditorViewOptions;
			const selectionController = existingViewport ? legacySelectionController : options!.selectionController;
			if (!selectionController) throw new TypeError('Editor view requires a selection controller');
			this.selectionController = selectionController;
			this.ownerId = options?.ownerId === undefined ? nextEditorViewId() : validateOwnerId(options.ownerId);
			this.viewport = existingViewport
				? existingViewport
				: this.own(new EditorViewport({
					...options!.viewport,
					container: options!.container,
					model: options!.model,
					lineHeight: options!.lineHeight,
					ariaLabel: options!.ariaLabel,
					selectionController,
				}));
			const viewOptions = existingViewport ? legacyOptions ?? {} : options!;
			validateViewOptions(viewOptions);
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

			// Language editing is contribution-owned. The view only borrows the adapter
			// while ViewController invokes it for the current input event.
			const languageEditing = viewOptions.languageEditing;
			this.editContext = this.own(createEditContext(this.viewport.element, {
				ariaLabel: viewOptions.ariaLabel,
				readOnly: selectionController.readOnly,
				textDirection: this.viewport.editorTextDirection,
				ownerId: this.ownerId,
				characterBoundsProvider: modelOffset => this.characterBoundsAt(modelOffset),
			}));
			this.input = this.editContext;
			this.element = this.editContext.element;
			this.textArea = this.editContext instanceof TextAreaEditContext
				? this.editContext.element
				: undefined;
			this.compositionController = this.own(new CompositionController(
				this.editContext,
				this.viewport,
				selectionController,
			));
			this.viewController = this.own(new ViewController(
				this.editContext,
				this.viewport,
				selectionController,
				this.compositionController,
				{ languageEditing, wordPattern: viewOptions.wordPattern },
			));
			this.onWillBeforeInput = this.viewController.onWillBeforeInput;
			this.onWillTextUpdate = this.viewController.onWillTextUpdate;
			this.onWillKeydown = this.viewController.onWillKeydown;
			this.onDidEdit = this.viewController.onDidEdit;

			if (this.editContext instanceof NativeEditContext) {
				this.own(new ScreenReaderSupport({
					element: this.editContext.element,
					model: this.viewport.textModel,
					viewport: this.viewport,
					selectionController,
					onDidFocus: this.editContext.onDidFocus,
					onDidBlur: this.editContext.onDidBlur,
					accessibilityService: viewOptions.accessibilityService,
					renderRichContent: viewOptions.renderRichScreenReaderContent,
					accessibilityPageSize: viewOptions.accessibilityPageSize,
					semanticTokenSource: viewOptions.semanticTokenSource,
					bracketColorizationSource: viewOptions.bracketColorizationSource,
					isComposing: () => this.compositionController.composing,
				}));
			}
			this.own(this.compositionController.onDidChange(composing => {
				if (!composing) this.synchronizeEditContext();
			}));
			if (this.editContext instanceof TextAreaEditContext) {
				this.own(new TextAreaAccessibilityController(
					this.editContext,
					this.viewport,
					selectionController,
					this.compositionController,
				));
			}
			this.defer(() => {
				this.viewport.element.classList.remove('input-focused');
				this.viewport.element.classList.remove('overtype');
			});
			this.own(addDisposableListener(this.viewport.element, 'focus', event => {
				if (event.target === this.viewport.element) this.focus();
			}));
			this.own(this.editContext.onDidFocus(() => {
				this.viewport.element.classList.add('input-focused');
			}));
			this.own(this.editContext.onDidBlur(() => {
				this.viewport.element.classList.remove('input-focused');
				this.editContext.clear();
			}));
			this.own(selectionController.onDidChange(() => this.synchronizeEditContext()));
			this.own(this.viewport.textModel.onDidChange(() => this.synchronizeEditContext()));
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
	revealPosition(position: Parameters<EditorViewport['revealPosition']>[0]): void {
		this.viewport.revealPosition(position);
	}

	clearInput(): void {
		this.editContext.clear();
	}

	private synchronizeEditContext(): void {
		const selection = this.selectionController.selections.primary;
		this.editContext.syncState({
			text: this.viewport.textModel.getText(),
			selectionStart: this.viewport.textModel.offsetAt(selection.range.start),
			selectionEnd: this.viewport.textModel.offsetAt(selection.range.end),
		});
		this.editContext.updateBounds(
			this.viewport.getPositionContentCoordinates(selection.active),
		);
	}

	private characterBoundsAt(modelOffset: number): EditContextCharacterBounds | undefined {
		const model = this.viewport.textModel;
		if (!Number.isSafeInteger(modelOffset) || modelOffset < 0 || modelOffset >= model.length) return undefined;
		const position = model.positionAt(modelOffset);
		const nextPosition = model.positionAt(Math.min(model.length, modelOffset + 1));
		const start = this.viewport.getPositionContentCoordinates(position);
		const end = this.viewport.getPositionContentCoordinates(nextPosition);
		const width = position.lineIndex === nextPosition.lineIndex
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
