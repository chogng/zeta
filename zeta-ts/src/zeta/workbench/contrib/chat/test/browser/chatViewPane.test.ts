import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { Emitter, type Event } from "../../../../../base/common/event.js";
import { toDisposable, type IDisposable } from "../../../../../base/common/lifecycle.js";
import type { IMenu, IMenuService } from "../../../../../platform/actions/common/menuService.js";
import type { ICommandService } from "../../../../../platform/commands/common/commands.js";
import type { IContextMenuService } from "../../../../../platform/contextview/browser/contextMenu.js";
import type { IChatService, ModelCatalogEntry, SkillCommandDefinition, SlashCommandDefinition, Thread, ThreadSubscription, ThreadUpdateEnvelope } from "../../../../services/chat/common/chatService.js";
import type { IWorkbenchLayoutService, WorkbenchPartId, WorkbenchPartVisibilityChangeEvent } from "../../../../services/layout/browser/layoutService.js";
import type { IActiveSessionThread, IUntitledChatSession, ModelRef, Session, SessionId, ThreadId } from "../../../../../sessions/services/sessions/common/session.js";
import type { ISessionsManagementService, SessionsManagementState } from "../../../../../sessions/services/sessions/common/sessionsManagementService.js";

const browserEnvironment = new JSDOM("<!doctype html><body></body>");
for (const [name, value] of Object.entries({
	window: browserEnvironment.window,
	document: browserEnvironment.window.document,
	Node: browserEnvironment.window.Node,
	Element: browserEnvironment.window.Element,
	HTMLElement: browserEnvironment.window.HTMLElement,
	Event: browserEnvironment.window.Event,
	InputEvent: browserEnvironment.window.InputEvent,
	KeyboardEvent: browserEnvironment.window.KeyboardEvent,
})) {
	Object.defineProperty(globalThis, name, { configurable: true, value });
}

const { ChatViewPane } = await import("../../browser/view/chatViewPane.js");
const { BrowserContextViewService } = await import("../../../../../platform/contextview/browser/contextViewService.js");

test.after(() => browserEnvironment.window.close());

test("opens a local Chat tab before the backend session request settles", () => {
	const document = browserEnvironment.window.document;
	const sessionService = new PendingSessionService();
	const layoutService = new VisibleAuxiliarybarLayoutService();
	using contextViewService = new BrowserContextViewService(document.body);
	using view = new ChatViewPane(
		document.body,
		{ id: "workbench.chat", title: "Chat" },
		unavailableChatService(),
		sessionService,
		emptyMenuService(),
		{} as IContextMenuService,
		contextViewService,
		{} as ICommandService,
		layoutService,
	);

	assert.equal(sessionService.untitledSessions.length, 1);
	assert.equal(view.element.querySelectorAll(".zeta-chat[data-untitled-session-id]").length, 1);
	assert.equal(view.element.querySelector<HTMLElement>(".zeta-chat-view-empty")?.hidden, true);
});

class PendingSessionService implements ISessionsManagementService {
	private readonly _onDidChange = new Emitter<void>();
	private readonly pendingInitialization = new Promise<void>(() => {});
	private _untitledSessions: readonly IUntitledChatSession[] = [];
	private _activeUntitledSessionId: string | undefined;

	readonly onDidChange = this._onDidChange.event;
	readonly sessions: readonly Session[] = [];
	readonly active: IActiveSessionThread | undefined = undefined;
	readonly state: SessionsManagementState = "loading";
	readonly error: string | undefined = undefined;

	get untitledSessions(): readonly IUntitledChatSession[] { return this._untitledSessions; }
	get activeUntitledSession(): IUntitledChatSession | undefined {
		return this._untitledSessions.find((session) => session.untitledSessionId === this._activeUntitledSessionId);
	}

	initialize(): Promise<void> { return this.pendingInitialization; }

	selectThread(_sessionId: SessionId, _threadId: ThreadId): void {}

	createUntitledSession(title = "New Chat"): IUntitledChatSession {
		const session = { untitledSessionId: `untitled-${this._untitledSessions.length + 1}`, title, model: undefined };
		this._untitledSessions = [session, ...this._untitledSessions];
		this._activeUntitledSessionId = session.untitledSessionId;
		this._onDidChange.fire();
		return session;
	}

	selectUntitledSession(untitledSessionId: string): void { this._activeUntitledSessionId = untitledSessionId; }
	discardUntitledSession(_untitledSessionId: string): void {}
	setUntitledSessionModel(_untitledSessionId: string, _model: ModelRef): void {}
	materializeUntitledSession(_untitledSessionId: string): Promise<IActiveSessionThread> { return Promise.reject(new Error("Backend is unavailable")); }
	promoteUntitledSession(_untitledSessionId: string, _active: IActiveSessionThread): void {}
	ensureActiveThread(): Promise<IActiveSessionThread> { return Promise.reject(new Error("Backend is unavailable")); }
	startNewSession(_title?: string): Promise<IActiveSessionThread> { return Promise.reject(new Error("Backend is unavailable")); }
	stopSession(_sessionId: SessionId): Promise<void> { return Promise.reject(new Error("Backend is unavailable")); }
	archiveSession(_sessionId: SessionId): Promise<void> { return Promise.reject(new Error("Backend is unavailable")); }
	setModel(_sessionId: SessionId, _model: ModelRef): Promise<void> { return Promise.reject(new Error("Backend is unavailable")); }
}

class VisibleAuxiliarybarLayoutService implements IWorkbenchLayoutService {
	private readonly _onDidChangePartVisibility = new Emitter<WorkbenchPartVisibilityChangeEvent>();

	readonly onDidChangePartVisibility = this._onDidChangePartVisibility.event;

	isPartVisible(partId: WorkbenchPartId): boolean { return partId === "auxiliarybar"; }
	showPart(_partId: WorkbenchPartId): void {}
	showParts(_partIds: readonly WorkbenchPartId[]): void {}
	hidePart(_partId: WorkbenchPartId): void {}
	hideParts(_partIds: readonly WorkbenchPartId[]): void {}
	getPartSize(_partId: WorkbenchPartId) { return { width: 0, height: 0 }; }
	resizePart(_partId: WorkbenchPartId, _dimension: { readonly width: number; readonly height: number }): void {}
}

function unavailableChatService(): IChatService {
	const pending = new Promise<never>(() => {});
	const neverEvent = <T>(): Event<T> => () => toDisposable(() => {});
	return {
		onDidUpdateThread: neverEvent<ThreadUpdateEnvelope>(),
		onDidBecomeReady: neverEvent<void>(),
		onDidChangeModels: neverEvent<void>(),
		onDidChangeSkills: neverEvent<void>(),
		listModels: () => pending as Promise<readonly ModelCatalogEntry[]>,
		listModelCatalog: () => pending as Promise<readonly ModelCatalogEntry[]>,
		refreshModels: () => pending as Promise<readonly ModelCatalogEntry[]>,
		isModelVisible: () => true,
		setModelVisible: () => pending as Promise<void>,
		listSlashCommands: () => pending as Promise<readonly SlashCommandDefinition[]>,
		listSkillCommands: () => pending as Promise<readonly SkillCommandDefinition[]>,
		readThread: (_sessionId: SessionId, _threadId: ThreadId) => pending as Promise<Thread>,
		subscribeThread: (_sessionId: SessionId, _threadId: ThreadId, _afterSequence: number) => pending as Promise<ThreadSubscription>,
		unsubscribeThread: (_sessionId: SessionId, _threadId: ThreadId) => pending as Promise<void>,
		startTurn: () => pending as Promise<void>,
		steerTurn: () => pending as Promise<void>,
		interruptTurn: () => pending as Promise<void>,
		resolveInteraction: () => pending as Promise<void>,
	};
}

function emptyMenuService(): IMenuService {
	const menu = Object.assign(toDisposable(() => {}), {
		onDidChange: () => toDisposable(() => {}),
		getActions: () => [],
	}) satisfies IMenu & IDisposable;
	return {
		createMenu: () => menu,
		getMenuActions: () => [],
	};
}
