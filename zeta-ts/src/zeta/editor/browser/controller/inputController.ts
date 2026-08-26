import { addDisposableListener } from '../../../base/browser/dom.js';
import { DisposableOwner, type IDisposable } from '../../../base/common/lifecycle.js';
import { type EditorSelectionController } from '../../common/cursor/editorSelectionController.js';
import { type EditorViewport } from '../view/editorViewport.js';
import { CompositionController } from './compositionController.js';
import { createEditContext } from './editContext/factory.js';
import { type EditContext, type EditContextCharacterBounds } from './editContext/editContext.js';
import { TextAreaAccessibilityController } from './editContext/textArea/textAreaAccessibilityController.js';
import { TextAreaEditContext } from './editContext/textArea/textAreaEditContext.js';
import { NativeEditContext } from './editContext/native/nativeEditContext.js';
import { ScreenReaderSupport } from './editContext/native/screenReaderSupport.js';
import { InputCommandController } from './inputCommandController.js';
import { InputCompletionController } from './inputCompletionController.js';
import {
	type InputCommandTransformer,
	type InputCompletionView,
	type InputControllerOptions,
	type InputIndentationOptions,
	type InputLanguageEditingAdapter,
} from './inputContracts.js';

export type {
	InputCommandContext,
	InputCommandTransformer,
	InputCompletionOptions,
	InputCompletionRequests,
	InputCompletionSession,
	InputCompletionView,
	InputCompletionViewFactory,
	InputControllerOptions,
	InputIndentationOptions,
	InputLanguageEditingAdapter,
	InputLanguageOptions,
	InputLanguageTypeCommand,
} from './inputContracts.js';

/**
 * Coordinates the browser input surface and the editor-local semantic input
 * controllers. Native DOM translation, composition, accessibility, and
 * command routing are deliberately separate collaborators.
 */
export class EditorInputController extends DisposableOwner {
	readonly element: HTMLElement;
	readonly editContext: EditContext;
	/** Compatibility alias for callers that still refer to the input surface. */
	readonly input: EditContext;
	readonly textArea: HTMLTextAreaElement | undefined;
	readonly compositionController: CompositionController;
	readonly completionWidget: InputCompletionView | undefined;
	private readonly languageEditing: InputLanguageEditingAdapter | undefined;
	private readonly wordPattern: (() => RegExp | undefined) | undefined;
	private readonly commandController: InputCommandController;
	private readonly completionController: InputCompletionController | undefined;

	constructor(
		private readonly viewport: EditorViewport,
		private readonly selectionController: EditorSelectionController,
		options: InputControllerOptions = {},
	) {
		super();
		validateIndentationOptions(options.indentation);
		validateAccessibilityPageSize(options.accessibilityPageSize);
		if (
			viewport.textModel !== selectionController.textModel ||
			(
				options.completion &&
				viewport.textModel !== options.completion.session.textModel
			) ||
			(
				options.completion?.requests &&
				(
					viewport.textModel !== options.completion.requests.service.textModel ||
					options.completion.session.resultStore !== options.completion.requests.service.results
				)
			)
		) {
			this.dispose();
			throw new TypeError('Stanza text input dependencies must share one text model and completion result store');
		}
		if (
			options.completion?.requests?.onRequestError !== undefined &&
			typeof options.completion.requests.onRequestError !== 'function'
		) {
			this.dispose();
			throw new TypeError('Stanza completion request error handler must be a function');
		}
		if (options.languageEditing && options.languageEditing.textModel !== viewport.textModel) {
			this.dispose();
			throw new TypeError('Stanza text input language editing must share its text model');
		}
		if (options.semanticTokenSource && options.semanticTokenSource.textModel !== viewport.textModel) {
			this.dispose();
			throw new TypeError('Stanza text input semantic tokens must share its text model');
		}
		if (options.bracketColorizationSource && options.bracketColorizationSource.textModel !== viewport.textModel) {
			this.dispose();
			throw new TypeError('Stanza text input bracket colorization must share its text model');
		}
		if (options.language && options.completion?.requests && options.completion.requests.languageId !== options.language.languageId) {
			this.dispose();
			throw new TypeError('Stanza text input language and completion request identities must match');
		}
		if (options.language && !options.languageEditing) {
			this.dispose();
			throw new Error('Text input language options require an explicit language-editing adapter');
		}
		this.languageEditing = options.languageEditing ? this.own(options.languageEditing) : undefined;
		this.wordPattern = options.wordPattern ?? (options.language ? () => options.language!.configurations.getLanguageConfiguration(options.language!.languageId).wordPattern : undefined);
		this.editContext = this.own(createEditContext(viewport.element, {
			ariaLabel: options.ariaLabel,
			readOnly: selectionController.readOnly,
			textDirection: viewport.editorTextDirection,
			characterBoundsProvider: modelOffset => this.characterBoundsAt(modelOffset),
		}));
		this.input = this.editContext;
		this.element = this.editContext.element;
		this.textArea = this.editContext instanceof TextAreaEditContext
			? this.editContext.element
			: undefined;
		this.completionController = options.completion ? this.own(new InputCompletionController(
			this.element,
			viewport,
			selectionController,
			options.completion,
		)) : undefined;
		this.completionWidget = this.completionController?.widget;
		this.compositionController = this.own(new CompositionController(
			this.editContext,
			viewport,
			selectionController,
		));
		if (this.editContext instanceof NativeEditContext) {
			this.own(new ScreenReaderSupport({
				element: this.editContext.element,
				model: viewport.textModel,
				viewport,
				selectionController,
				onDidFocus: this.editContext.onDidFocus,
				onDidBlur: this.editContext.onDidBlur,
				accessibilityService: options.accessibilityService,
				renderRichContent: options.renderRichScreenReaderContent,
				accessibilityPageSize: options.accessibilityPageSize,
				semanticTokenSource: options.semanticTokenSource,
				bracketColorizationSource: options.bracketColorizationSource,
				isComposing: () => this.compositionController.composing,
			}));
		}
		this.own(this.compositionController.onDidChange(composing => {
			if (!composing) this.synchronizeEditContext();
		}));
		this.commandController = this.own(new InputCommandController(
			this.editContext,
			viewport,
			selectionController,
			this.compositionController,
			{
				languageEditing: this.languageEditing,
				wordPattern: this.wordPattern,
				...(this.completionController ? { completion: this.completionController } : {}),
			},
		));
		if (this.editContext instanceof TextAreaEditContext) {
			this.own(new TextAreaAccessibilityController(
				this.editContext,
				viewport,
				selectionController,
				this.compositionController,
			));
		}
		this.defer(() => {
			viewport.element.classList.remove('input-focused');
			viewport.element.classList.remove('overtype');
		});

		this.own(addDisposableListener(viewport.element, 'focus', event => {
			if (event.target === viewport.element) this.focus();
		}));
		this.own(this.editContext.onDidFocus(() => {
			viewport.element.classList.add('input-focused');
		}));
		this.own(this.editContext.onDidBlur(() => {
			viewport.element.classList.remove('input-focused');
			this.editContext.clear();
		}));
		this.own(selectionController.onDidChange(() => this.synchronizeEditContext()));
		this.own(viewport.textModel.onDidChange(() => this.synchronizeEditContext()));
		this.synchronizeEditContext();
		this.editContext.connect();
	}

	focus(): void {
		this.editContext.focus();
	}

	get overtyping(): boolean {
		return this.commandController.overtyping;
	}

	registerCommandTransformer(transformer: InputCommandTransformer): IDisposable {
		return this.commandController.registerCommandTransformer(transformer);
	}

	/** Toggles this editor instance's transient overtype input mode. */
	toggleOvertype(): boolean {
		return this.commandController.toggleOvertype();
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
			: Math.max(1, this.viewport.measureTextWidth(" "));
		return Object.freeze({
			left: Math.min(start.left, end.left),
			top: start.top,
			width,
			height: start.height,
		});
	}
}

function validateIndentationOptions(options: InputIndentationOptions | undefined): void {
	if (options === undefined) return;
	if (typeof options !== 'object' || options === null) throw new TypeError('Editor indentation options must be an object');
	if (options.kind !== undefined && options.kind !== 'tabs' && options.kind !== 'spaces') throw new TypeError('Unknown editor indentation kind');
	if (options.tabSize !== undefined && (!Number.isSafeInteger(options.tabSize) || options.tabSize < 1 || options.tabSize > 32)) throw new RangeError('Editor tab size must be a safe integer between 1 and 32');
}

function validateAccessibilityPageSize(pageSize: number | undefined): void {
	if (pageSize !== undefined && (!Number.isSafeInteger(pageSize) || pageSize < 1 || pageSize > 10_000)) {
		throw new RangeError('Editor accessibility page size must be a safe integer between 1 and 10000');
	}
}
