import type {
	IDimension,
} from "../../../../base/browser/geometry.js";
import type {
	IDisposable,
} from "../../../../base/common/lifecycle.js";
import type { URI } from "../../../../base/common/uri.js";
import type {
	IConfigurationService,
} from "../../../../platform/configuration/common/configurationService.js";
import { type ITextFileService } from "../../../services/textfile/common/textFileService.js";
import type { IFileService } from "../../../../platform/files/common/files.js";
import { type ITextMateService } from "../../../services/textMate/common/textMateService.js";
import { type ILanguageFeaturesService } from "../../../services/language/common/languageFeaturesService.js";
import type { EditorInput } from "./editorInput.js";
import type { IDiffApi } from "../../../../platform/diff/common/diffApi.js";
import type { IInstantiationService } from "../../../../platform/instantiation/common/instantiation.js";
import type { ISyntaxApi } from "../../../../platform/syntax/common/syntaxApi.js";
import type { IWorkingCopyService, IWorkingCopy } from "../../../services/workingCopy/common/workingCopyService.js";
import type { IDocumentCollaborationApi } from "../../../../platform/collaboration/common/documentCollaborationApi.js";
import type { IServerEventApi } from "../../../../platform/app-server/common/appServerApi.js";
import type { TextRange } from "../../../../editor/common/core/text.js";
import type { LanguageLocation } from "../../../../editor/contrib/gotoSymbol/common/languageNavigation.js";
import type { LanguageWorkspaceEdit } from "../../../../editor/common/languages/languageWorkspaceEdit.js";
import type { ILanguageDiagnosticsService } from "../../../../editor/common/services/languageDiagnosticsService.js";
import type { EditorLineGutterDecoration } from "../../../../editor/browser/viewparts/margin/lineGutterDecoration.js";
import type { OwnedDecorationSource } from "../../../../editor/browser/viewparts/decorations/decorationPresentation.js";
import type { TextModel } from "../../../../editor/common/model/textModel.js";
import type { IKeybindingService } from "../../../../platform/keybinding/common/keybinding.js";
import type { IKeybindingsResourceService } from "../../../../platform/keybinding/common/keybindingsResource.js";
import type { IKeyboardLayoutService } from "../../../../platform/keyboardLayout/common/keyboardLayout.js";
import type { IContextKeyService } from "../../../../platform/contextkey/common/contextkey.js";
import type { IContextMenuProvider } from "../../../../base/browser/contextmenu.js";
import type { IMenuService } from "../../../../platform/actions/common/menuService.js";

export enum EditorPaneVisibility {
	Hidden,
	Visible,
}

/**
 * One editor implementation hosted by the central Editor Part.
 *
 * Implementations create their DOM exactly once in the supplied parent.
 * `setInput` may resolve asynchronously, must observe the abort signal, and
 * must reject when the input cannot be opened. The host owns the pane and
 * disposes it after hiding it.
 */
export interface IEditorPane extends IDisposable {
	readonly id: string;
	/** Optional format-specific document exposed through the shared Workbench lifecycle. */
	readonly workingCopy?: IWorkingCopy;

	create(parent: HTMLElement): void;
	setInput(input: EditorInput, signal: AbortSignal): Promise<void>;
	clearInput(): void;
	layout(dimension: IDimension): void;
	setVisible(visibility: EditorPaneVisibility): void;
	focus(): void;
	/** Reveals an editor-owned text range when this pane supports text navigation. */
	revealRange?(range: TextRange): void;
	/** Persists the active editor's current resource when that editor is writable. */
	save?(): Promise<void>;
	/** Serializes and persists the active document to a new resource when supported. */
	saveAs?(resource: URI): Promise<void>;
}

export interface EditorPaneCreationOptions {
	/** The input used to choose a profile-specific pane implementation. */
	readonly input?: EditorInput;
	readonly configurationService?: IConfigurationService;
	readonly contextKeyService?: IContextKeyService;
	/** Group-scoped action services for pane-owned menus and toolbars. */
	readonly actionServices?: {
		readonly menuService: IMenuService;
		readonly contextMenuProvider: IContextMenuProvider;
		readonly contextKeyService?: IContextKeyService;
	};
	readonly keybindingService?: IKeybindingService;
	readonly keybindingsResourceService?: IKeybindingsResourceService;
	readonly keyboardLayoutService?: IKeyboardLayoutService;
	readonly fileService?: IFileService;
	readonly textFileService?: ITextFileService;
	readonly textMateService?: ITextMateService;
	readonly languageFeaturesService?: ILanguageFeaturesService;
	readonly diffApi?: IDiffApi;
	readonly instantiationService?: IInstantiationService;
	readonly syntaxApi?: ISyntaxApi;
	readonly languageDiagnosticsService?: ILanguageDiagnosticsService;
	readonly documentCollaborationApi?: IDocumentCollaborationApi;
	readonly serverEvents?: IServerEventApi;
	readonly workingCopyService?: IWorkingCopyService;
	readonly onSave?: () => Promise<void | boolean>;
	readonly onOpenLocation?: (location: LanguageLocation) => void | Promise<void>;
	readonly onApplyWorkspaceEdit?: (edit: LanguageWorkspaceEdit) => void | Promise<void>;
	readonly createLineGutterDecorations?: (resource: URI) => readonly EditorLineGutterDecoration[];
	readonly createDecorationSources?: (resource: URI, model: TextModel) => readonly OwnedDecorationSource[];
}

export enum EditorPaneMatch {
	None,
	Optional,
	Default,
}

/**
 * Declares how one editor implementation is matched and constructed.
 *
 * Descriptors must keep `canOpen` pure. Product contribution modules register
 * descriptors before the Workbench creates its Editor Part.
 */
export interface IEditorPaneDescriptor {
	readonly id: string;
	readonly name: string;
	canOpen(input: EditorInput): EditorPaneMatch;
	create(options: EditorPaneCreationOptions): IEditorPane;
}
