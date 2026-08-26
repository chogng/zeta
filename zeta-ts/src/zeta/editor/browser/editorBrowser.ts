import { type IDimension } from "../../base/browser/geometry.js";
import { isNonEmptyArray } from "../../base/common/arrays.js";
import { type Event } from "../../base/common/event.js";
import { DisposableOwner, type IDisposable } from "../../base/common/lifecycle.js";
import { isFiniteNumber, isSafeInteger } from "../../base/common/numbers.js";
import { type ISyntaxApi } from "../../platform/syntax/common/syntaxApi.js";
import { type EditorResourceInput } from "../common/editorResource.js";
import { type EditorSelectionController } from "../common/cursor/editorSelectionController.js";
import { type TextPosition, type TextRange } from "../common/core/text.js";
import { type LanguageCompletionWorkerFactory } from "../common/languages/completion/languageCompletionService.js";
import { type SyntaxWorkerFactory } from "../common/languages/syntax/syntaxService.js";
import { type ILanguageFeaturesService } from "../common/services/languageService.js";
import { type TextModelReference } from "../common/services/textModelService.js";
import { type EditorIndentationOptions } from "../common/editorIndentation.js";
import { type EditorInputController } from "./controller/inputController.js";
import { type CodeEditorWidget } from "./widget/codeEditor/codeEditorWidget.js";
import { type EditorHitTarget } from "../common/viewModel/pointerHitTest.js";
import { type EditorActiveLineHighlight, type EditorMinimap, type EditorRuler, type EditorTextDirection, type EditorViewport, type EditorViewportPresentation } from "./view/editorViewport.js";
import { type EditorLineWrapping } from "./viewModel/visualLineProjection.js";
import { type LanguageLocation } from "../contrib/gotoSymbol/common/languageNavigation.js";
import { type LanguageWorkspaceEdit } from "../common/languages/languageWorkspaceEdit.js";
import { type ILanguageDiagnosticsService } from "../common/services/languageDiagnosticsService.js";
import { type EditorLineGutterDecoration } from "./viewparts/margin/lineGutterDecoration.js";
import { type OwnedDecorationSource } from "./viewparts/decorations/decorationPresentation.js";
import { type IDiffApi } from "../../platform/diff/common/diffApi.js";
import { type IInstantiationService } from "../../platform/instantiation/common/instantiation.js";
import { type IAccessibilityService } from "../../platform/accessibility/common/accessibility.js";
import { type TabFocus } from "./config/tabFocus.js";
import { EditorBrowserRuntime } from "./editorBrowserRuntime.js";

export interface EditorContextMenuRequest {
	readonly position: TextPosition;
	readonly target: EditorHitTarget | undefined;
	readonly clientX: number;
	readonly clientY: number;
}

export interface EditorTextViewPositionState {
	readonly lineIndex: number;
	readonly columnIndex: number;
}

export interface EditorTextViewSelectionState {
	readonly anchor: EditorTextViewPositionState;
	readonly active: EditorTextViewPositionState;
}

/** JSON-safe instance state persisted by a Workbench text-editor pane. */
export interface EditorTextViewState {
	readonly selections: readonly EditorTextViewSelectionState[];
	readonly primarySelectionIndex: number;
	readonly scrollPosition: {
		readonly left: number;
		readonly top: number;
	};
}

export function isEditorTextViewState(value: unknown): value is EditorTextViewState {
	if (!value || typeof value !== "object") return false;
	const state = value as Partial<EditorTextViewState>;
	if (!isNonEmptyArray(state.selections)) return false;
	if (!isSafeInteger(state.primarySelectionIndex) || state.primarySelectionIndex! < 0 || state.primarySelectionIndex! >= state.selections.length) return false;
	if (!isViewScrollPosition(state.scrollPosition)) return false;
	return state.selections.every(selection => isViewPosition(selection?.anchor) && isViewPosition(selection?.active));
}

/** Defaults applied whenever the editor-local Find and Replace widget opens. */
export interface EditorFindOptions {
	readonly seedSearchStringFromSelection?: boolean;
	readonly autoFindInSelection?: boolean;
	readonly loop?: boolean;
	readonly matchCase?: boolean;
	readonly wholeWord?: boolean;
	readonly regularExpression?: boolean;
}

export interface EditorBrowserOptions {
	readonly container: HTMLElement;
	readonly input: EditorResourceInput;
	readonly languageId: string;
	/** Optional host-scoped Tab-focus state shared by multiple editor instances. */
	readonly tabFocus?: TabFocus;
	/** Optional shared language registrations and providers for this editor host. */
	readonly languageFeaturesService?: ILanguageFeaturesService;
	/** Optional Rust-backed syntax facts used for parser-grade fold ranges. */
	readonly syntaxApi?: ISyntaxApi;
	/** Optional Rust-backed line diff API exposed to editor-local contributions. */
	readonly diffApi?: IDiffApi;
	/** Window-scoped constructor service for runtime editor contributions. */
	readonly instantiationService?: IInstantiationService;
	/** Optional accessibility policy used by native screen-reader content. */
	readonly accessibilityService?: IAccessibilityService;
	/** Chooses line-structured content for native screen-reader projection. */
	readonly renderRichScreenReaderContent?: boolean;
	/** Controls how many logical lines one native screen-reader page exposes. */
	readonly accessibilityPageSize?: number;
	/** Optional host service that synchronizes open models and supplies push diagnostics. */
	readonly languageDiagnosticsService?: ILanguageDiagnosticsService;
	readonly modelReference: TextModelReference;
	readonly syntaxWorkerFactory?: SyntaxWorkerFactory;
	readonly completionWorkerFactory?: LanguageCompletionWorkerFactory;
	readonly languageSupport?: IDisposable;
	readonly onDidChangeLanguageSupport?: Event<void>;
	readonly whenLanguageSupportReady?: () => Promise<unknown>;
	readonly onLanguageError?: (error: unknown) => void;
	readonly onSave?: () => Promise<void | boolean>;
	readonly onRevert?: () => Promise<void>;
	readonly indentation?: EditorIndentationOptions;
	readonly lineWrapping?: EditorLineWrapping;
	readonly fontFamily?: string;
	readonly fontSize?: number;
	readonly lineHeight?: number;
	readonly fontLigatures?: boolean;
	readonly minimap?: EditorMinimap;
	readonly activeLineHighlight?: EditorActiveLineHighlight;
	readonly showLineNumbers?: boolean;
	readonly rulers?: readonly EditorRuler[];
	readonly showIndentationGuides?: boolean;
	readonly bracketPairColorization?: boolean;
	readonly stickyScroll?: boolean;
	readonly suggestions?: boolean;
	readonly inlineCompletions?: boolean;
	readonly parameterHints?: boolean;
	readonly inlayHints?: boolean;
	readonly codeLens?: boolean;
	readonly formatOnSave?: boolean;
	readonly find?: EditorFindOptions;
	/** Applies a single LF at the save boundary when the document has content and no final LF. */
	readonly insertFinalNewLine?: boolean;
	/** Browser paragraph direction for this editor browser's DOM projection. */
	readonly textDirection?: EditorTextDirection;
	readonly presentation?: EditorViewportPresentation;
	/** Host-owned link opening callback; the editor never opens external targets directly. */
	readonly onOpenLink?: (target: string) => void | Promise<void>;
	/** Host-owned context-menu composition; the editor supplies only hit-test data. */
	readonly onShowContextMenu?: (request: EditorContextMenuRequest) => void | Promise<void>;
	/** Host-owned execution for provider commands such as code lenses. */
	readonly onExecuteEditorCommand?: (id: string, args: readonly unknown[] | undefined) => void | Promise<void>;
	/** Host-owned cross-resource navigation; same-resource reveal remains editor-owned. */
	readonly onOpenLocation?: (location: LanguageLocation) => void | Promise<void>;
	/** Host-owned multi-resource edit transaction. */
	readonly onApplyWorkspaceEdit?: (edit: LanguageWorkspaceEdit) => void | Promise<void>;
	/** Host-contributed gutter presentation; feature semantics remain outside the editor core. */
	readonly lineGutterDecorations?: readonly EditorLineGutterDecoration[];
	/** Host-created decoration sources whose lifetime transfers to this editor part. */
	readonly decorationSources?: readonly OwnedDecorationSource[];
	readonly placeholder?: string;
	readonly showUnicodeHighlights?: boolean;
	readonly fontZoom?: { readonly initialScale?: number };
}

/** Runtime created by one statically selected line-editor contribution bundle. */
export interface IEditorBrowserRuntime extends IDisposable {
	readonly onDidChange: Event<void>;
	readonly codeEditor: CodeEditorWidget;
	readonly viewport: EditorViewport;
	readonly selections: EditorSelectionController;
	readonly input: EditorInputController;
	announceAccessibilityStatus(message: string): void;
	layout(dimension: IDimension): void;
	focus(): void;
	getValue(): string;
	setValue(value: string): void;
	revealRange(range: TextRange): void;
	getViewState(): EditorTextViewState;
	restoreViewState(state: EditorTextViewState): void;
	readonly isDirty: boolean;
	readonly hasExternalChange: boolean;
	save(): Promise<void>;
	revert(): Promise<void>;
}

/** Browser composition root for the line editor. */
export class EditorBrowser extends DisposableOwner implements IEditorBrowserRuntime {
	private readonly runtime: IEditorBrowserRuntime;
	readonly onDidChange: Event<void>;
	readonly codeEditor: CodeEditorWidget;
	readonly viewport: EditorViewport;
	readonly selections: EditorSelectionController;
	readonly input: EditorInputController;

	constructor(options: EditorBrowserOptions) {
		super();
		try {
			this.runtime = this.own(new EditorBrowserRuntime(options));
			this.onDidChange = this.runtime.onDidChange;
			this.codeEditor = this.runtime.codeEditor;
			this.viewport = this.runtime.viewport;
			this.selections = this.runtime.selections;
			this.input = this.runtime.input;
		} catch (error) {
			this.dispose();
			throw error;
		}
	}

	layout(dimension: IDimension): void { this.runtime.layout(dimension); }
	announceAccessibilityStatus(message: string): void { this.runtime.announceAccessibilityStatus(message); }
	focus(): void { this.runtime.focus(); }
	getValue(): string { return this.runtime.getValue(); }
	setValue(value: string): void { this.runtime.setValue(value); }
	revealRange(range: TextRange): void { this.runtime.revealRange(range); }
	getViewState(): EditorTextViewState { return this.runtime.getViewState(); }
	restoreViewState(state: EditorTextViewState): void { this.runtime.restoreViewState(state); }
	get isDirty(): boolean { return this.runtime.isDirty; }
	get hasExternalChange(): boolean { return this.runtime.hasExternalChange; }
	save(): Promise<void> { return this.runtime.save(); }
	revert(): Promise<void> { return this.runtime.revert(); }
}

function isViewPosition(value: unknown): value is EditorTextViewPositionState {
	if (!value || typeof value !== "object") return false;
	const position = value as Partial<EditorTextViewPositionState>;
	return isSafeInteger(position.lineIndex) && position.lineIndex! >= 0 && isSafeInteger(position.columnIndex) && position.columnIndex! >= 0;
}

function isViewScrollPosition(value: unknown): value is EditorTextViewState["scrollPosition"] {
	if (!value || typeof value !== "object") return false;
	const position = value as Partial<EditorTextViewState["scrollPosition"]>;
	return isFiniteNumber(position.left) && position.left! >= 0 && isFiniteNumber(position.top) && position.top! >= 0;
}
