import { type IDisposable } from '../../../base/common/lifecycle.js';
import { type EditorEditCommand } from '../../common/commands/editorEditCommand.js';
import { type TextSelectionSet } from '../../common/core/selection.js';
import { type TextModelChange } from '../../common/core/text.js';
import { type EditorSelectionController } from '../../common/cursor/editorSelectionController.js';
import { type LanguageCompletionService } from '../../common/languages/completion/languageCompletionService.js';
import { type LanguageCompletionResult } from '../../common/languages/completion/languageCompletions.js';
import { type LanguageCompletionContext } from '../../common/languages/completion/languageCompletionProviders.js';
import { type VersionedLanguageResultStore } from '../../common/languages/languageResultStore.js';
import { type LanguageConfigurationSource } from '../../common/languages/languageConfiguration.js';
import { type LanguageLexicalContextSource } from '../../common/languages/languageLexicalContext.js';
import { type TextModel } from '../../common/model/textModel.js';
import { type EditorViewport } from '../view/editorViewport.js';
import { type IAccessibilityService } from '../../../platform/accessibility/common/accessibility.js';
import { type BracketColorizationSource, type SemanticTokenSource } from '../viewparts/semanticTokens/semanticTokenPresentation.js';

export interface InputControllerOptions {
	readonly ariaLabel?: string;
	/** Optional platform accessibility policy used by native screen-reader support. */
	readonly accessibilityService?: IAccessibilityService;
	/** Selects line-structured content for the native screen-reader mirror. */
	readonly renderRichScreenReaderContent?: boolean;
	/** Controls how many logical lines one native screen-reader page exposes. */
	readonly accessibilityPageSize?: number;
	/** Optional syntax presentation sources used by the native rich screen-reader mirror. */
	readonly semanticTokenSource?: SemanticTokenSource;
	readonly bracketColorizationSource?: BracketColorizationSource;
	readonly completion?: InputCompletionOptions;
	readonly indentation?: InputIndentationOptions;
	readonly language?: InputLanguageOptions;
	readonly languageEditing?: InputLanguageEditingAdapter;
	readonly wordPattern?: () => RegExp | undefined;
}

export interface InputCommandContext {
	readonly inputType: string;
}

/** Extends one native input command before it becomes an atomic model transaction. */
export type InputCommandTransformer = (command: EditorEditCommand, context: InputCommandContext) => EditorEditCommand;

export interface InputIndentationOptions {
	readonly kind?: 'tabs' | 'spaces';
	readonly tabSize?: number;
}

export interface InputLanguageOptions {
	readonly languageId: string;
	readonly configurations: LanguageConfigurationSource;
	readonly lexicalContext?: LanguageLexicalContextSource;
}

export interface InputLanguageTypeCommand {
	readonly command: EditorEditCommand;
	readonly insertedText: boolean;
	afterExecute?(change: TextModelChange): void;
}

/** Optional language-aware editing seam implemented by bracket and indentation contributions. */
export interface InputLanguageEditingAdapter extends IDisposable {
	readonly textModel: TextModel;
	createTypeCommand(selections: TextSelectionSet, text: string): InputLanguageTypeCommand | undefined;
	createEnterCommand(selections: TextSelectionSet): EditorEditCommand | undefined;
	createBackspaceCommand(selections: TextSelectionSet): EditorEditCommand | undefined;
}

export interface InputCompletionOptions {
	readonly session: InputCompletionSession;
	readonly requests?: InputCompletionRequests;
	/** Explicit presentation supplied by the active Suggest contribution. */
	readonly viewFactory: InputCompletionViewFactory;
}

/** Structural completion session contract consumed by native text input. */
export interface InputCompletionSession {
	readonly textModel: TextModel;
	readonly resultStore: VersionedLanguageResultStore<LanguageCompletionResult>;
	readonly state: { readonly isIncomplete: boolean } | undefined;
	acceptSelectedWithCommitCharacter(commitCharacter?: string): boolean;
	cancel(): boolean;
	cancelSnippetPlaceholderNavigation(): boolean;
	selectNextSnippetChoice(): boolean;
	selectPreviousSnippetChoice(): boolean;
	selectNextSnippetPlaceholder(): boolean;
	selectPreviousSnippetPlaceholder(): boolean;
}

export interface InputCompletionView extends IDisposable {
	readonly element: HTMLElement;
	readonly visible: boolean;
}

export type InputCompletionViewFactory = (element: HTMLElement, viewport: EditorViewport, selections: EditorSelectionController, session: InputCompletionSession) => InputCompletionView;

export interface InputCompletionRequests {
	readonly service: LanguageCompletionService;
	readonly languageId: string;
	readonly onRequestError?: (error: unknown) => void;
}

export interface InputCompletionRequestDelegate {
	readonly session: InputCompletionSession | undefined;
	readonly requests: InputCompletionRequests | undefined;
	readonly readIsIncomplete: () => boolean;
	readonly requestAfterInsert: (insertedText: string, refreshIncomplete: boolean) => void;
	readonly requestCompletion: (context: LanguageCompletionContext) => void;
}
