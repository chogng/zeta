import { type IDisposable } from "../../base/common/lifecycle.js";
import { type Event } from '../../base/common/event.js';
import { type ServiceConstructionDescriptor } from "../../platform/instantiation/common/instantiation.js";
import { type CursorsController } from "../common/cursor/cursor.js";
import { type LanguageConfigurationSource } from "../common/languages/ownedLanguageConfigurationContributions.js";
import { type TextModel } from "../common/model/textModel.js";
import { type DocumentTextStyleAttributes } from "../common/model/documentSchema.js";
import type { IEditorLanguageFeaturesService } from '../common/services/languageFeatures.js';
import type { IResolvedSemanticTokensService } from '../common/services/resolvedSemanticTokens.js';
import type { ISemanticTokensStylingService } from '../common/services/semanticTokensStyling.js';
import { type DocumentCollaborationInvite } from "../common/services/documentCollaborationService.js";
import { type DocumentCollaborationMember } from "../common/services/documentCollaborationService.js";
import { type DocumentCollaborationRoomRole } from "../common/services/documentCollaborationService.js";
import { type ConfiguredCodeEditorOptions } from './configuredCodeEditor.js';
import { type EditorView } from './editorView.js';
import { type View } from "./view.js";
import { type DecorationSource } from "./viewParts/decorations/decorations.js";
import { type EditorLineVisibilitySource } from "../common/viewModel/viewModelLines.js";
import { type LanguageLexicalContextSource } from "../common/languages/languageLexicalContext.js";
import { type BracketColorizationSource, type SemanticTokenSource } from "./viewParts/viewLines/viewLine.js";
import { type TabFocus } from "./config/tabFocus.js";
import { type IVersionedEditorWorkerClient } from "./services/editorWorkerService.js";
import { TriggerInlineEditCommandsRegistry } from './triggerInlineEditCommandsRegistry.js';

export interface EditorCommandEvent {
	readonly commandId: string;
}

export interface EditorCommandMetadata {
	readonly id: string;
	readonly canTriggerInlineEdits?: boolean;
}

export type EditorCommandExecutor = <T>(commandId: string, operation: () => T) => T;

/** Stable text-model mount point exposed to optional editor extensions. */
export interface EditorCapability<T> {
	readonly id: string;
	readonly _value?: T;
}

/** Pre-widget assembly seam for extensions that supply model projection inputs. */
export interface TextEditorContributionConfigurationContext {
	readonly kind: "text";
	readonly options: ConfiguredCodeEditorOptions;
	readonly model: TextModel;
	readonly editorWorker: IVersionedEditorWorkerClient;
	readonly languageId: string;
	readonly languageFeaturesService: IEditorLanguageFeaturesService;
	readonly semanticTokensStylingService: ISemanticTokensStylingService;
	readonly resolvedSemanticTokensService: IResolvedSemanticTokensService;
	readonly configurations: LanguageConfigurationSource;
	readonly selections: CursorsController;
	readonly tabFocus: TabFocus;
	readonly onLanguageError: (error: unknown) => void;
	readonly getCapability: <T>(capability: EditorCapability<T>) => T;
	readonly getOptionalCapability: <T>(capability: EditorCapability<T>) => T | undefined;
	readonly provideCapability: <T>(capability: EditorCapability<T>, value: T) => void;
	readonly addDecorationSource: (source: DecorationSource) => void;
	readonly setLineProjection: (projection: { readonly visibilitySource: EditorLineVisibilitySource }) => void;
	readonly setSemanticTokenSource: (source: SemanticTokenSource) => void;
	readonly setBracketColorizationSource: (source: BracketColorizationSource) => void;
	readonly setLanguageLexicalContext: (source: LanguageLexicalContextSource) => void;
	readonly register: <T extends IDisposable>(value: T) => T;
}

export interface TextEditorContributionContext {
	readonly kind: "text";
	readonly options: ConfiguredCodeEditorOptions;
	readonly model: TextModel;
	readonly editorWorker: IVersionedEditorWorkerClient;
	readonly languageId: string;
	readonly languageFeaturesService: IEditorLanguageFeaturesService;
	readonly configurations: LanguageConfigurationSource;
	readonly view: EditorView;
	readonly viewport: View;
	readonly selections: CursorsController;
	readonly tabFocus: TabFocus;
	readonly onLanguageError: (error: unknown) => void;
	readonly onDidExecuteCommand: Event<EditorCommandEvent>;
	readonly executeCommand: EditorCommandExecutor;
	readonly getCapability: <T>(capability: EditorCapability<T>) => T;
	readonly getOptionalCapability: <T>(capability: EditorCapability<T>) => T | undefined;
	readonly registerBeforeSave: (hook: () => void | Promise<void>) => IDisposable;
	readonly register: <T extends IDisposable>(value: T) => T;
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
	setState(state: "unavailable" | "inactive" | "connecting" | "connected" | "resyncRequired" | "error", options?: { readonly roomId?: string; readonly message?: string; readonly principalId?: string; readonly canManageMembers?: boolean }): void;
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
	readonly onStartCollaboration: (roomId: string | undefined) => Promise<DocumentCollaborationStartResult>;
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
export const enum EditorContributionInstantiation {
	Eager,
	AfterFirstRender,
	BeforeFirstInteraction,
	Eventually,
	Lazy,
}

export interface TextEditorRuntimeContributionRegistration {
	readonly descriptor: ServiceConstructionDescriptor<TextEditorRuntimeContribution>;
	readonly instantiation: EditorContributionInstantiation;
}

/** Installs one statically selected capability at its supported editor mount point. */
export interface TextEditorCapabilityContribution {
	readonly id: string;
	readonly commands?: readonly EditorCommandMetadata[];
	configure?(context: TextEditorContributionConfigurationContext): void;
	install?(context: EditorContributionContext): void;
	/** Optional constructor-backed runtime for services that need editor-local state and DI. */
	readonly runtime?: TextEditorRuntimeContributionRegistration;
}

const contributions: TextEditorCapabilityContribution[] = [];
const contributionIds = new Set<string>();

/** Registers one flat editor extension through a Workbench mode bundle side effect. */
export function registerTextEditorCapabilityContribution(contribution: TextEditorCapabilityContribution): void {
	if (!contribution || !contribution.id?.trim() || (typeof contribution.configure !== "function" && typeof contribution.install !== "function" && !contribution.runtime)) throw new TypeError("Editor contribution is invalid");
	if (contribution.runtime && (!contribution.runtime.descriptor?.ctor || !isInstantiation(contribution.runtime.instantiation))) {
		throw new TypeError("Editor runtime contribution is invalid");
	}
	if (contributionIds.has(contribution.id)) throw new RangeError(`Duplicate editor contribution '${contribution.id}'`);
	for (const command of contribution.commands ?? []) {
		if (!command || typeof command.id !== 'string' || command.id.trim().length === 0) throw new TypeError('Editor command metadata is invalid');
		if (command.canTriggerInlineEdits) TriggerInlineEditCommandsRegistry.registerCommand(command.id);
	}
	contributionIds.add(contribution.id);
	contributions.push(contribution);
}

export function getTextEditorCapabilityContributions(): readonly TextEditorCapabilityContribution[] {
	return contributions;
}

function isInstantiation(value: EditorContributionInstantiation): boolean {
	return value === EditorContributionInstantiation.Eager
		|| value === EditorContributionInstantiation.AfterFirstRender
		|| value === EditorContributionInstantiation.BeforeFirstInteraction
		|| value === EditorContributionInstantiation.Eventually
		|| value === EditorContributionInstantiation.Lazy;
}
