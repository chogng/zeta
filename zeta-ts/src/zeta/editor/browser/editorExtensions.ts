import { type IDisposable } from "../../base/common/lifecycle.js";
import { type SyncDescriptor } from "../../platform/instantiation/common/instantiation.js";
import { type EditorSelectionController } from "../common/cursor/editorSelectionController.js";
import { type LanguageConfigurationSource } from "../common/languages/languageConfiguration.js";
import { type TextModel } from "../common/model/textModel.js";
import { type DocumentTextStyleAttributes } from "../common/model/documentSchema.js";
import { type ILanguageFeaturesService } from "../common/services/languageService.js";
import { type DocumentCollaborationInvite } from "../common/services/documentCollaborationService.js";
import { type DocumentCollaborationMember } from "../common/services/documentCollaborationService.js";
import { type DocumentCollaborationRoomRole } from "../common/services/documentCollaborationService.js";
import { type DocumentCollaborationTarget } from "../common/services/documentCollaborationService.js";
import { type EditorBrowserOptions } from "./editorBrowser.js";
import { type EditorLanguageEditingAdapter, type EditorView } from "./view.js";
import { type EditorViewport } from "./view.js";
import { type DecorationSource } from "./viewparts/decorations/decorationPresentation.js";
import { type EditorLineGutterDecoration } from "./viewparts/margin/lineGutterDecoration.js";
import { type EditorLineVisibilitySource } from "../common/viewModel/viewModelLines.js";
import { type LanguageLexicalContextSource } from "../common/languages/languageLexicalContext.js";
import { type BracketColorizationSource, type SemanticTokenSource } from "./viewparts/semanticTokens/semanticTokenPresentation.js";
import { type TabFocus } from "./config/tabFocus.js";

/** Stable text-model mount point exposed to optional editor extensions. */
export interface EditorCapability<T> {
	readonly id: string;
	readonly _value?: T;
}

/** Pre-widget assembly seam for extensions that supply model projection inputs. */
export interface TextEditorContributionConfigurationContext {
	readonly kind: "text";
	readonly options: EditorBrowserOptions;
	readonly model: TextModel;
	readonly languageId: string;
	readonly languageFeaturesService: ILanguageFeaturesService;
	readonly configurations: LanguageConfigurationSource;
	readonly selections: EditorSelectionController;
	readonly tabFocus: TabFocus;
	readonly onLanguageError: (error: unknown) => void;
	readonly getCapability: <T>(capability: EditorCapability<T>) => T;
	readonly getOptionalCapability: <T>(capability: EditorCapability<T>) => T | undefined;
	readonly provideCapability: <T>(capability: EditorCapability<T>, value: T) => void;
	readonly addDecorationSource: (source: DecorationSource) => void;
	readonly addLineGutterDecoration: (decoration: EditorLineGutterDecoration) => void;
	readonly setLineProjection: (projection: { readonly visibilitySource: EditorLineVisibilitySource; readonly gutterDecoration?: EditorLineGutterDecoration }) => void;
	readonly setSemanticTokenSource: (source: SemanticTokenSource) => void;
	readonly setBracketColorizationSource: (source: BracketColorizationSource) => void;
	readonly setLanguageLexicalContext: (source: LanguageLexicalContextSource) => void;
	readonly setLanguageEditing: (adapter: EditorLanguageEditingAdapter) => void;
	readonly own: <T extends IDisposable>(value: T) => T;
}

export interface TextEditorContributionContext {
	readonly kind: "text";
	readonly options: EditorBrowserOptions;
	readonly model: TextModel;
	readonly languageId: string;
	readonly languageFeaturesService: ILanguageFeaturesService;
	readonly configurations: LanguageConfigurationSource;
	readonly view: EditorView;
	readonly viewport: EditorViewport;
	readonly selections: EditorSelectionController;
	readonly tabFocus: TabFocus;
	readonly onLanguageError: (error: unknown) => void;
	readonly getCapability: <T>(capability: EditorCapability<T>) => T;
	readonly getOptionalCapability: <T>(capability: EditorCapability<T>) => T | undefined;
	readonly registerBeforeSave: (hook: () => void | Promise<void>) => IDisposable;
	readonly own: <T extends IDisposable>(value: T) => T;
}

export interface DocumentFormattingState {
	readonly context: "none" | "text" | "code";
	readonly readOnly: boolean;
	readonly bold: boolean;
	readonly italic: boolean;
	readonly fontFamily: "sans" | "serif" | "monospace" | undefined;
	readonly fontSize: number | undefined;
	readonly checkedDocumentActionIds: ReadonlySet<string>;
}

export interface DocumentFormattingContribution extends IDisposable {
	readonly element: HTMLElement;
	setState(state: DocumentFormattingState): void;
}

export interface DocumentCollaborationStartResult {
	readonly roomId: string;
	readonly principalId: string | undefined;
	readonly canManageMembers: boolean;
}

export interface DocumentCollaborationContribution extends IDisposable {
	readonly element: HTMLElement;
	setState(state: "unavailable" | "inactive" | "connecting" | "connected" | "resyncRequired" | "error", options?: { readonly roomId?: string; readonly message?: string; readonly target?: DocumentCollaborationTarget; readonly principalId?: string; readonly canManageMembers?: boolean }): void;
}

/** Stable structured-TextModel mount point exposed to the same flat extension registry. */
export interface DocumentEditorContributionContext {
	readonly kind: "document";
	readonly container: HTMLElement;
	readonly documentActions: readonly { readonly id: string; readonly label: string }[];
	readonly onToggleMark: (markType: "strong" | "em") => void;
	readonly onSetTextStyle: (attrs: DocumentTextStyleAttributes) => void;
	readonly onClearTextStyle: () => void;
	readonly onRunDocumentAction: (actionId: string) => void;
	readonly onStartCollaboration: (roomId: string | undefined, target: DocumentCollaborationTarget) => Promise<DocumentCollaborationStartResult>;
	readonly onStopCollaboration: () => void;
	readonly onInviteCollaborator: (displayName: string, role: DocumentCollaborationRoomRole) => Promise<DocumentCollaborationInvite>;
	readonly onListCollaborators: () => Promise<readonly DocumentCollaborationMember[]>;
	readonly onRotateCollaboratorAccessToken: (principalId: string) => Promise<DocumentCollaborationInvite>;
	readonly onRevokeCollaborator: (principalId: string) => Promise<void>;
	readonly setFormattingContribution: (contribution: DocumentFormattingContribution) => void;
	readonly setCollaborationContribution: (contribution: DocumentCollaborationContribution) => void;
}

export type EditorContributionContext = TextEditorContributionContext | DocumentEditorContributionContext;

/** Runtime object constructed for one text editor with editor-local state followed by injected services. */
export interface TextEditorRuntimeContribution extends IDisposable {}

/** Controls when a constructor-backed extension joins one text editor's lifetime. */
export enum EditorContributionInstantiation {
	Eager = "eager",
	AfterFirstRender = "afterFirstRender",
	BeforeFirstInteraction = "beforeFirstInteraction",
	Eventually = "eventually",
	Lazy = "lazy",
}

export interface TextEditorRuntimeContributionRegistration {
	readonly descriptor: SyncDescriptor<TextEditorRuntimeContribution>;
	readonly instantiation: EditorContributionInstantiation;
}

/** Installs one statically selected capability at its supported editor mount point. */
export interface EditorContribution {
	readonly id: string;
	configure?(context: TextEditorContributionConfigurationContext): void;
	install?(context: EditorContributionContext): void;
	/** Optional constructor-backed runtime for services that need editor-local state and DI. */
	readonly runtime?: TextEditorRuntimeContributionRegistration;
}

const contributions: EditorContribution[] = [];
const contributionIds = new Set<string>();

/** Registers one flat editor extension through a Workbench mode bundle side effect. */
export function registerEditorContribution(contribution: EditorContribution): void {
	if (!contribution || !contribution.id?.trim() || (typeof contribution.configure !== "function" && typeof contribution.install !== "function" && !contribution.runtime)) throw new TypeError("Editor contribution is invalid");
	if (contribution.runtime && (!contribution.runtime.descriptor?.ctor || !isInstantiation(contribution.runtime.instantiation))) {
		throw new TypeError("Editor runtime contribution is invalid");
	}
	if (contributionIds.has(contribution.id)) throw new RangeError(`Duplicate editor contribution '${contribution.id}'`);
	contributionIds.add(contribution.id);
	contributions.push(contribution);
}

export function getEditorContributions(): readonly EditorContribution[] {
	return contributions;
}

function isInstantiation(value: EditorContributionInstantiation): boolean {
	return value === EditorContributionInstantiation.Eager
		|| value === EditorContributionInstantiation.AfterFirstRender
		|| value === EditorContributionInstantiation.BeforeFirstInteraction
		|| value === EditorContributionInstantiation.Eventually
		|| value === EditorContributionInstantiation.Lazy;
}
