import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import type { ModelRef, ServerNotification, Session, SessionCommandParams, SessionCreateParams, SessionModelSetParams, SessionThreadCreateParams, Thread, TurnStartParams } from "../generated/app-server/types.js";
import type { ZetaRendererApi } from "../src/zeta/platform/app-server/common/renderer-api.js";
import type { IAction } from "../src/zeta/base/common/actions.js";
import { TAB_CLOSE_ACTION_ID } from "../src/zeta/base/browser/ui/tablist/tabList.js";
import { lxiconsLibrary } from "../src/zeta/base/common/lxiconsLibrary.js";
import { toDisposable } from "../src/zeta/base/common/lifecycle.js";
import { MenuId } from "../src/zeta/platform/actions/common/actions.js";
import { MenuService } from "../src/zeta/platform/actions/common/menuService.js";
import type { IContextMenuService } from "../src/zeta/platform/contextview/browser/contextMenu.js";
import { ServiceCollection } from "../src/zeta/platform/instantiation/common/instantiation.js";
import { IQuickInputService } from "../src/zeta/platform/quickinput/common/quickInput.js";
import { CommandService } from "../src/zeta/workbench/services/commands/common/commandService.js";
import type { ViewPaneContainer } from "../src/zeta/workbench/browser/parts/views/viewPaneContainer.js";
import { ViewContainerLocation, WorkbenchViewRegistry } from "../src/zeta/workbench/common/views.js";
import { ChatPaneModel } from "../src/zeta/workbench/contrib/chat/browser/pane/chatPaneModel.js";
import { CHAT_VIEW_CONTAINER_ID, CHAT_VIEW_ID, MOVE_CHAT_TO_EDITOR_COMMAND_ID, MOVE_CHAT_TO_NEW_WINDOW_COMMAND_ID, NEW_CHAT_COMMAND_ID, OPEN_CHAT_BROWSER_COMMAND_ID, OPEN_CHAT_SETTINGS_COMMAND_ID, SHOW_CHAT_HISTORY_COMMAND_ID, TOGGLE_AGENT_SIDEBAR_COMMAND_ID } from "../src/zeta/workbench/contrib/chat/common/chat.js";
import { ISettingsService } from "../src/zeta/workbench/services/preferences/common/settings.js";
import { IWorkbenchSessionService, WorkbenchSessionService } from "../src/zeta/workbench/services/sessions/common/sessionService.js";
import { IViewsService, ViewsService } from "../src/zeta/workbench/services/views/browser/viewsService.js";
import { ContextKeyService, IContextKeyService } from "../src/zeta/platform/contextkey/common/contextkey.js";
import { ViewDescriptorService } from "../src/zeta/workbench/services/views/common/viewDescriptorService.js";
import { WorkbenchQuickInputService } from "../src/zeta/workbench/services/quickinput/browser/quickInputService.js";

const browserEnvironment = new JSDOM("<!doctype html><body></body>");
for (const [name, value] of Object.entries({
  window: browserEnvironment.window,
  document: browserEnvironment.window.document,
  Node: browserEnvironment.window.Node,
  Element: browserEnvironment.window.Element,
  HTMLElement: browserEnvironment.window.HTMLElement,
  Event: browserEnvironment.window.Event,
  MouseEvent: browserEnvironment.window.MouseEvent,
})) {
  Object.defineProperty(globalThis, name, {
    configurable: true,
    value,
  });
}
const { registerChatViews } = await import(
  "../src/zeta/workbench/contrib/chat/browser/chat.contribution.js"
);
const { ChatViewPane } = await import(
  "../src/zeta/workbench/contrib/chat/browser/view/chatViewPane.js"
);
await import(
  "../src/zeta/workbench/contrib/preferences/browser/preferences.contribution.js"
);
test.after(() => {
  browserEnvironment.window.close();
  for (const name of [
    "window",
    "document",
    "Node",
    "Element",
    "HTMLElement",
    "Event",
    "MouseEvent",
  ]) {
    Reflect.deleteProperty(globalThis, name);
  }
});

test("Chat contribution owns the fixed Auxiliary Bar view", () => {
  const registry = new WorkbenchViewRegistry();

  registerChatViews(registry);

  assert.equal(
    registry.getDefaultViewContainer(ViewContainerLocation.AuxiliaryBar)?.id,
    CHAT_VIEW_CONTAINER_ID,
  );
  assert.deepEqual(
    registry.getViews(CHAT_VIEW_CONTAINER_ID).map((view) => view.id),
    [CHAT_VIEW_ID],
  );
});

test("Chat title separates Session tabs from its action toolbar", async () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const fake = fakeApi({
    sessions: [
      session("session-1", "thread-1"),
      session("session-2", "thread-2"),
    ],
  });
  const api = fake.api;
  using sessions = new WorkbenchSessionService(api);
  using contextKeys = new ContextKeyService();
  using viewDescriptors = new ViewDescriptorService({
    contextKeyService: contextKeys,
    registry: new WorkbenchViewRegistry(),
  });
  const services = new ServiceCollection();
  let openedSettingsSection: string | undefined;
  const settings: ISettingsService = {
    onDidChangeVisibility: () => toDisposable(() => {}),
    onDidChangeActiveSection: () => toDisposable(() => {}),
    isOpen: false,
    activeSectionId: "general",
    open: (sectionId) => {
      openedSettingsSection = sectionId;
    },
    close() {},
  };
  services.set(ISettingsService, settings);
  services.set(IContextKeyService, contextKeys);
  using commands = new CommandService(services);
  const menuService = new MenuService(commands, contextKeys);
  let shownContextMenuActions: readonly IAction[] = [];
  const contextMenuService = {
    showContextMenu: (options: { readonly actions?: readonly IAction[] }) => {
      shownContextMenuActions = options.actions ?? [];
    },
  } as unknown as IContextMenuService;
  using pane = new ChatViewPane(
    {
      id: CHAT_VIEW_ID,
      title: "Chat",
      ownerDocument: dom.window.document,
    },
    api,
    sessions,
    menuService,
    contextMenuService,
    viewDescriptors,
    contextKeys,
    commands,
  );
  dom.window.document.body.append(pane.element);

  await sessions.initialize();
  await nextTask();

  const title = pane.element.querySelector(".zeta-chat-title-control");
  const tablist = title?.querySelector(
    ".zeta-chat-tabs-control .zeta-action-bar",
  );
  const toolbars = title?.querySelectorAll(
    ".zeta-chat-title-actions > .zeta-action-bar",
  );
  const toolbar = toolbars?.[0];
  const layoutToolbar = toolbars?.[1];
  assert.equal(tablist?.getAttribute("role"), "tablist");
  assert.equal(toolbar?.getAttribute("role"), "toolbar");
  assert.equal(toolbar?.classList.contains("zeta-toolbar"), true);
  assert.equal(toolbars?.length, 2);
  assert.equal(
    tablist?.closest(".zeta-chat-tabs-control")?.nextElementSibling,
    toolbar?.parentElement,
  );
  assert.deepEqual(
    [...toolbar?.querySelectorAll<HTMLElement>("[data-action-id]") ?? []]
      .map((item) => item.dataset.actionId),
    [
      NEW_CHAT_COMMAND_ID,
      SHOW_CHAT_HISTORY_COMMAND_ID,
      "zeta.toolbar.moreActions",
    ],
  );
  assert.deepEqual(
    [...toolbar?.querySelectorAll<HTMLButtonElement>("button") ?? []]
      .map((button) => button.title),
    ["New Chat", "Show Chat History", "More Actions"],
  );
  assert.equal(
    toolbar?.querySelectorAll(".zeta-action-view-item.icon").length,
    3,
  );
  assert.ok(toolbar?.querySelector(".zeta-button-label"));
  assert.ok(toolbar?.querySelector("svg.zeta-icon"));
  assert.ok(layoutToolbar?.querySelector(
    `[data-action-id="${TOGGLE_AGENT_SIDEBAR_COMMAND_ID}"]`,
  ));
  const agentSidebar = pane.element.querySelector<HTMLElement>(
    "[data-part='agentSidebar']",
  );
  assert.equal(agentSidebar?.hidden, true);
  await commands.executeCommand(TOGGLE_AGENT_SIDEBAR_COMMAND_ID);
  assert.equal(agentSidebar?.hidden, false);
  await commands.executeCommand(TOGGLE_AGENT_SIDEBAR_COMMAND_ID);
  assert.equal(agentSidebar?.hidden, true);
  const chatActions = menuService.getMenuActions(MenuId.ChatTitle)
    .filter(([group]) => group !== "navigation")
    .flatMap(([, actions]) => actions);
  assert.deepEqual(
    chatActions.map((action) => ({
      id: action.id,
      label: action.label,
      enabled: action.enabled,
      icon: action.icon,
    })),
    [
      {
        id: OPEN_CHAT_BROWSER_COMMAND_ID,
        label: "Open Browser",
        enabled: false,
        icon: lxiconsLibrary.browserWeb,
      },
      {
        id: MOVE_CHAT_TO_EDITOR_COMMAND_ID,
        label: "Move Chat to Editor Area",
        enabled: false,
        icon: lxiconsLibrary.layoutPanel,
      },
      {
        id: MOVE_CHAT_TO_NEW_WINDOW_COMMAND_ID,
        label: "Move Chat to New Window",
        enabled: false,
        icon: lxiconsLibrary.linkExternal,
      },
      {
        id: OPEN_CHAT_SETTINGS_COMMAND_ID,
        label: "Chat Settings",
        enabled: true,
        icon: lxiconsLibrary.settings,
      },
    ],
  );
  await chatActions[3]?.run();
  assert.equal(openedSettingsSection, "chat");
  const tabs = tablist?.querySelectorAll<HTMLButtonElement>("[role='tab']");
  assert.equal(tabs?.length, 2);
  assert.deepEqual(
    [...(tabs ?? [])].map((tab) => tab.getAttribute("aria-selected")),
    ["true", "false"],
  );
  const panel = pane.element.querySelector("[role='tabpanel']");
  assert.equal(panel?.id, tabs?.[0]?.getAttribute("aria-controls"));
  assert.equal(panel?.getAttribute("aria-labelledby"), tabs?.[0]?.id);
  assert.equal(
    pane.element.querySelector(
      ".zeta-chat-tabs-control .zeta-scrollable-element",
    )?.getAttribute("data-scroll-direction"),
    "horizontal",
  );
  assert.equal(
    pane.element.querySelector(
      ".zeta-chat-transcript-scrollable",
    )?.getAttribute("data-scroll-direction"),
    "vertical",
  );
  const chatPanes = pane.element.querySelectorAll<HTMLElement>(".zeta-chat-pane-host > .zeta-chat");
  assert.equal(chatPanes.length, 2);
  for (const chatPane of chatPanes) {
    assert.equal(chatPane.childElementCount, 2);
    assert.ok(chatPane.firstElementChild?.classList.contains("zeta-chat-input-widget"));
    assert.ok(chatPane.lastElementChild?.classList.contains("zeta-chat-list-widget"));
    const inputToolbar = chatPane.querySelector<HTMLElement>(".zeta-chat-input-toolbar");
    assert.equal(inputToolbar?.getAttribute("role"), "toolbar");
    assert.deepEqual(
      [...inputToolbar?.querySelectorAll<HTMLElement>("[data-action-id]") ?? []].map((item) => item.dataset.actionId),
      [
        "zeta.chat.input.mode",
        "zeta.chat.input.model",
        "zeta.chat.input.attachment",
        "zeta.chat.input.send",
      ],
    );
    assert.equal(inputToolbar?.querySelector<HTMLButtonElement>("[data-action-id='zeta.chat.input.mode'] button")?.textContent, "Agent");
    assert.equal(inputToolbar?.querySelector<HTMLButtonElement>("[data-action-id='zeta.chat.input.model'] button")?.textContent, "Model");
    assert.equal(inputToolbar?.querySelector<HTMLButtonElement>("[data-action-id='zeta.chat.input.attachment'] button")?.disabled, true);
    assert.equal(inputToolbar?.querySelector<HTMLButtonElement>("[data-action-id='zeta.chat.input.send'] button")?.disabled, true);
  }
  const firstChatPane = chatPanes[0]!;
  firstChatPane.querySelector<HTMLButtonElement>("[data-action-id='zeta.chat.input.mode'] button")?.click();
  assert.deepEqual(shownContextMenuActions.map((action) => action.label), ["Agent", "Plan", "Debug", "Multitask", "Ask"]);
  shownContextMenuActions[1]?.run();
  assert.equal(firstChatPane.querySelector<HTMLButtonElement>("[data-action-id='zeta.chat.input.mode'] button")?.textContent, "Plan");
  assert.deepEqual([...chatPanes].map((chatPane) => chatPane.hidden), [false, true]);
  const composerInputs = [...chatPanes].map((chatPane) => {
    const input = chatPane.querySelector<HTMLTextAreaElement>(".zeta-alpha-editor-input");
    assert.ok(input);
    return input;
  });
  typeAlphaText(dom.window, composerInputs[0], "First draft");
  assert.equal(firstChatPane.querySelector<HTMLButtonElement>("[data-action-id='zeta.chat.input.send'] button")?.disabled, false);

  tabs?.[1]?.click();
  assert.equal(sessions.active?.session.sessionId, "session-2");
  assert.equal(sessions.active?.threadId, "thread-2");
  assert.deepEqual(
    [...pane.element.querySelectorAll<HTMLElement>("[role='tab']")]
      .map((tab) => tab.getAttribute("aria-selected")),
    ["false", "true"],
  );
  assert.deepEqual([...chatPanes].map((chatPane) => chatPane.hidden), [true, false]);
  typeAlphaText(dom.window, composerInputs[1], "Second draft");
  pane.element.querySelectorAll<HTMLButtonElement>("[role='tab']")[0]?.click();
  assert.equal(sessions.active?.session.sessionId, "session-1");
  assert.deepEqual([...chatPanes].map((chatPane) => chatPane.hidden), [false, true]);
  assert.equal(chatPanes[0]?.querySelector(".zeta-alpha-editor-line-text")?.textContent, "First draft");
  assert.equal(chatPanes[1]?.querySelector(".zeta-alpha-editor-line-text")?.textContent, "Second draft");

  const closeButtons = pane.element.querySelectorAll<HTMLButtonElement>(
    `[data-action-id="${TAB_CLOSE_ACTION_ID}"] button`,
  );
  assert.equal(closeButtons.length, 2);
  assert.deepEqual(
    [...closeButtons].map((button) => button.title),
    ["Close Session session-1", "Close Session session-2"],
  );
  closeButtons[0]?.click();
  await nextTask();

  assert.deepEqual(
    fake.archiveRequests.map(({ sessionId, expectedSequence }) => ({
      sessionId,
      expectedSequence,
    })),
    [{ sessionId: "session-1", expectedSequence: 2 }],
  );
  assert.equal(sessions.active?.session.sessionId, "session-2");
  assert.deepEqual(
    [...pane.element.querySelectorAll<HTMLElement>("[role='tab']")]
      .map((tab) => ({
        label: tab.textContent,
        selected: tab.getAttribute("aria-selected"),
      })),
    [{ label: "Session session-2", selected: "true" }],
  );
  assert.equal(
    pane.element.querySelector<HTMLElement>(".zeta-chat-pane-host > .zeta-chat")
      ?.dataset.sessionId,
    "session-2",
  );

  dom.window.close();
});

test("an empty Session list opens a local draft pane and persists it on its first send", async () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const createdSession = session("session-1", undefined, "New Chat");
  const attachedSession = session("session-1", "thread-1", "New Chat");
  const fake = fakeApi({
    sessions: [],
    createSession: createdSession,
    createThread: {
      session: attachedSession,
      threadId: "thread-1",
    },
  });
  const api = fake.api;
  const services = new ServiceCollection();
  using sessions = new WorkbenchSessionService(api);
  using contextKeys = new ContextKeyService();
  using viewDescriptors = new ViewDescriptorService({
    contextKeyService: contextKeys,
    registry: new WorkbenchViewRegistry(),
  });
  services.set(IWorkbenchSessionService, sessions);
  services.set(IViewsService, {
    openView: () => undefined,
    focusView: () => true,
  });
  using commands = new CommandService(services);
  const menuService = new MenuService(commands, contextKeys);
  const contextMenuService = {
    showContextMenu: () => undefined,
  } as unknown as IContextMenuService;
  using pane = new ChatViewPane(
    {
      id: CHAT_VIEW_ID,
      title: "Chat",
      ownerDocument: dom.window.document,
    },
    api,
    sessions,
    menuService,
    contextMenuService,
    viewDescriptors,
    contextKeys,
    commands,
  );
  dom.window.document.body.append(pane.element);

  await sessions.initialize();
  await nextTask();

  const tabs = pane.element.querySelectorAll<HTMLButtonElement>("[role='tab']");
  assert.equal(tabs.length, 1);
  assert.deepEqual(
    [...tabs].map((tab) => ({
      label: tab.textContent,
      selected: tab.getAttribute("aria-selected"),
    })),
    [
      { label: "New Chat", selected: "true" },
    ],
  );
  assert.equal(pane.element.querySelector<HTMLElement>(".zeta-chat-view-empty")?.hidden, true);
  assert.equal(fake.createSessionRequests.length, 0);
  assert.equal(fake.createThreadRequests.length, 0);
  assert.equal(sessions.sessions.length, 0);
  assert.equal(sessions.drafts.length, 1);
  const draftPane = pane.element.querySelector<HTMLElement>("[role='tabpanel']");
  assert.ok(draftPane?.dataset.draftId);
  const input = draftPane.querySelector<HTMLTextAreaElement>(".zeta-alpha-editor-input");
  assert.ok(input);
  typeAlphaText(dom.window, input, "Hello from a draft");
  input.dispatchEvent(new dom.window.KeyboardEvent("keydown", {
    bubbles: true,
    cancelable: true,
    key: "Enter",
  }));
  await waitFor(() => fake.turnStartRequests.length === 1);

  assert.equal(fake.createSessionRequests.length, 1);
  assert.equal(fake.createThreadRequests.length, 1);
  assert.equal(fake.turnStartRequests.length, 1);
  assert.equal(sessions.drafts.length, 0);
  assert.equal(sessions.active?.session.sessionId, "session-1");
  assert.equal(sessions.active?.threadId, "thread-1");
  assert.equal(
    pane.element.querySelector<HTMLElement>("[role='tabpanel']")?.dataset.sessionId,
    "session-1",
  );
  assert.equal(
    pane.element.querySelector("[role='tabpanel']")?.getAttribute(
      "aria-labelledby",
    ),
    tabs[0]?.id,
  );

  dom.window.close();
});

test("the New Chat slash command opens a local draft", async () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const initialSession = session("session-1", "thread-1", "First Chat");
  const fake = fakeApi({ sessions: [initialSession] });
  const services = new ServiceCollection();
  using sessions = new WorkbenchSessionService(fake.api);
  using contextKeys = new ContextKeyService();
  using viewDescriptors = new ViewDescriptorService({
    contextKeyService: contextKeys,
    registry: new WorkbenchViewRegistry(),
  });
  services.set(IWorkbenchSessionService, sessions);
  let focusedView: string | undefined;
  services.set(IViewsService, {
    openView: () => undefined,
    focusView: (viewId) => {
      focusedView = viewId;
      return true;
    },
  });
  using commands = new CommandService(services);
  const menuService = new MenuService(commands, contextKeys);
  const contextMenuService = {
    showContextMenu: () => undefined,
  } as unknown as IContextMenuService;
  using pane = new ChatViewPane(
    {
      id: CHAT_VIEW_ID,
      title: "Chat",
      ownerDocument: dom.window.document,
    },
    fake.api,
    sessions,
    menuService,
    contextMenuService,
    viewDescriptors,
    contextKeys,
    commands,
  );
  dom.window.document.body.append(pane.element);

  await sessions.initialize();
  await nextTask();

  const input = pane.element.querySelector<HTMLTextAreaElement>(".zeta-chat:not([hidden]) .zeta-alpha-editor-input");
  assert.ok(input);
  typeAlphaText(dom.window, input, "/new");
  assert.equal(pane.element.querySelector("[data-action-id='zeta.chat.input.command'] button")?.textContent, "Command");
  input.dispatchEvent(new dom.window.KeyboardEvent("keydown", {
    bubbles: true,
    cancelable: true,
    key: "Enter",
  }));
  await waitFor(() => pane.element.querySelectorAll("[role='tab']").length === 2);

  assert.deepEqual(
    [...pane.element.querySelectorAll<HTMLElement>("[role='tab']")].map((tab) => ({
      label: tab.textContent,
      selected: tab.getAttribute("aria-selected"),
    })),
    [
      { label: "New Chat", selected: "true" },
      { label: "First Chat", selected: "false" },
    ],
  );
  assert.equal(fake.createSessionRequests.length, 0);
  assert.equal(fake.createThreadRequests.length, 0);
  assert.equal(sessions.drafts.length, 1);
  assert.equal(focusedView, CHAT_VIEW_ID);

  dom.window.close();
});

test("failed first send keeps the local draft and its message", async () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const fake = fakeApi({
    sessions: [],
    createSessionError: new Error("Cannot create Session"),
  });
  const services = new ServiceCollection();
  using sessions = new WorkbenchSessionService(fake.api);
  using contextKeys = new ContextKeyService();
  using viewDescriptors = new ViewDescriptorService({
    contextKeyService: contextKeys,
    registry: new WorkbenchViewRegistry(),
  });
  services.set(IWorkbenchSessionService, sessions);
  services.set(IViewsService, {
    openView: () => undefined,
    focusView: () => true,
  });
  using commands = new CommandService(services);
  const menuService = new MenuService(commands, contextKeys);
  const contextMenuService = {
    showContextMenu: () => undefined,
  } as unknown as IContextMenuService;
  using pane = new ChatViewPane(
    {
      id: CHAT_VIEW_ID,
      title: "Chat",
      ownerDocument: dom.window.document,
    },
    fake.api,
    sessions,
    menuService,
    contextMenuService,
    viewDescriptors,
    contextKeys,
    commands,
  );
  dom.window.document.body.append(pane.element);

  await sessions.initialize();
  await nextTask();

  const input = pane.element.querySelector<HTMLTextAreaElement>(".zeta-alpha-editor-input");
  assert.ok(input);
  typeAlphaText(dom.window, input, "Keep this draft");
  input.dispatchEvent(new dom.window.KeyboardEvent("keydown", {
    bubbles: true,
    cancelable: true,
    key: "Enter",
  }));
  await waitFor(() => pane.element.querySelector(".zeta-alpha-editor-line-text")?.textContent === "Keep this draft");

  assert.equal(fake.createSessionRequests.length, 1);
  assert.equal(sessions.sessions.length, 0);
  assert.equal(sessions.drafts.length, 1);
  assert.equal(pane.element.querySelector(".zeta-alpha-editor-line-text")?.textContent, "Keep this draft");
  assert.equal(pane.element.querySelector<HTMLElement>("[role='tabpanel']")?.dataset.draftId, sessions.drafts[0]?.draftId);
  assert.match(pane.element.querySelector<HTMLElement>(".zeta-chat-status")?.textContent ?? "", /Cannot create Session/);

  dom.window.close();
});

test("one Session retains one Chat pane while its selected Thread changes", async () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const multiThreadSession: Session = {
    ...session("session-1", "thread-1", "One Chat"),
    threads: [
      { threadId: "thread-1", origin: { type: "root" }, status: "active" },
      {
        threadId: "thread-2",
        origin: {
          type: "fork",
          parentThreadId: "thread-1",
          parentSequence: 1,
        },
        status: "active",
      },
    ],
  };
  const api = fakeApi({ sessions: [multiThreadSession] }).api;
  using sessions = new WorkbenchSessionService(api);
  using contextKeys = new ContextKeyService();
  using viewDescriptors = new ViewDescriptorService({
    contextKeyService: contextKeys,
    registry: new WorkbenchViewRegistry(),
  });
  using commands = new CommandService(new ServiceCollection());
  const menuService = new MenuService(commands, contextKeys);
  const contextMenuService = {
    showContextMenu: () => undefined,
  } as unknown as IContextMenuService;
  using pane = new ChatViewPane(
    {
      id: CHAT_VIEW_ID,
      title: "Chat",
      ownerDocument: dom.window.document,
    },
    api,
    sessions,
    menuService,
    contextMenuService,
    viewDescriptors,
    contextKeys,
    commands,
  );
  dom.window.document.body.append(pane.element);

  await sessions.initialize();
  await nextTask();

  assert.equal(pane.element.querySelectorAll("[role='tab']").length, 1);
  const chatPane = pane.element.querySelector<HTMLElement>(".zeta-chat-pane-host > .zeta-chat");
  assert.ok(chatPane);
  assert.equal(chatPane.dataset.sessionId, "session-1");
  assert.equal(chatPane.dataset.threadId, "thread-1");

  sessions.selectThread("session-1", "thread-2");
  await nextTask();

  assert.strictEqual(
    pane.element.querySelector(".zeta-chat-pane-host > .zeta-chat"),
    chatPane,
  );
  assert.equal(pane.element.querySelectorAll("[role='tab']").length, 1);
  assert.equal(chatPane.dataset.threadId, "thread-2");
  dom.window.close();
});

test("Chat history selects an active Thread through Quick Pick", async () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const api = fakeApi({
    sessions: [
      session("session-1", "thread-1", "First Chat"),
      session("session-2", "thread-2", "Second Chat"),
    ],
  }).api;
  const services = new ServiceCollection();
  using sessions = new WorkbenchSessionService(api);
  using contextKeys = new ContextKeyService();
  using quickInput = new WorkbenchQuickInputService({
    container: dom.window.document.body,
    contextKeyService: contextKeys,
  });
  let focusedView: string | undefined;
  services.set(IWorkbenchSessionService, sessions);
  services.set(IQuickInputService, quickInput);
  services.set(IViewsService, {
    openView: () => undefined,
    focusView: (viewId) => {
      focusedView = viewId;
      return true;
    },
  });
  using commands = new CommandService(services);
  await sessions.initialize();

  await commands.executeCommand(SHOW_CHAT_HISTORY_COMMAND_ID);
  assert.deepEqual(
    [...dom.window.document.querySelectorAll(
      ".zeta-quick-pick-row-label",
    )].map((label) => label.textContent),
    ["First Chat", "Second Chat"],
  );
  const input = dom.window.document.querySelector<HTMLInputElement>(
    ".zeta-quick-pick-input input",
  );
  assert.ok(input);
  input.dispatchEvent(new dom.window.KeyboardEvent("keydown", {
    bubbles: true,
    cancelable: true,
    key: "ArrowDown",
  }));
  input.dispatchEvent(new dom.window.KeyboardEvent("keydown", {
    bubbles: true,
    cancelable: true,
    key: "Enter",
  }));

  assert.equal(sessions.active?.session.sessionId, "session-2");
  assert.equal(sessions.active?.threadId, "thread-2");
  assert.equal(focusedView, CHAT_VIEW_ID);
  assert.equal(
    dom.window.document.querySelector(".zeta-quick-pick"),
    null,
  );
  dom.window.close();
});

test("ViewsService resolves, opens, and focuses contributed views", () => {
  const registry = new WorkbenchViewRegistry();
  registerChatViews(registry);
  using contextKeys = new ContextKeyService();
  using descriptors = new ViewDescriptorService({
    contextKeyService: contextKeys,
    registry,
  });
  let focused = 0;
  let opened = 0;
  const view = {
    id: CHAT_VIEW_ID,
    focus: () => focused++,
    isVisible: () => true,
    setVisible: () => undefined,
  };
  const service = new ViewsService({
    viewDescriptorService: descriptors,
    openViewContainer: (container) => {
      assert.equal(container.id, CHAT_VIEW_CONTAINER_ID);
      return {
        openView: (viewId: string) => {
          assert.equal(viewId, CHAT_VIEW_ID);
          opened++;
          return view;
        },
      } as unknown as ViewPaneContainer;
    },
  });

  assert.equal(service.focusView(CHAT_VIEW_ID), true);
  assert.equal(opened, 1);
  assert.equal(focused, 1);
  assert.equal(service.openView("missing"), undefined);
});

test("WorkbenchSessionService restores and creates active Threads", async () => {
  const initialSession = session("session-1", "thread-1");
  const createdSession = session("session-2");
  const attachedSession = session("session-2", "thread-2");
  const api = fakeApi({
    sessions: [initialSession],
    createSession: createdSession,
    createThread: {
      session: attachedSession,
      threadId: "thread-2",
    },
  }).api;
  using service = new WorkbenchSessionService(api);

  await service.initialize();
  assert.equal(service.active?.threadId, "thread-1");
  assert.equal(service.active?.session.title, "Session session-1");

  const active = await service.startNewSession("Another");
  assert.equal(active.threadId, "thread-2");
  assert.equal(service.sessions[0].sessionId, "session-2");
  assert.equal(service.state, "ready");
});

test("WorkbenchSessionService archives a Session and selects the next active one", async () => {
  const first = session("session-1", "thread-1");
  const second = session("session-2", "thread-2");
  const fake = fakeApi({ sessions: [first, second] });
  using service = new WorkbenchSessionService(fake.api);

  await service.initialize();
  await service.archiveSession("session-1");

  assert.deepEqual(
    fake.archiveRequests.map(({ sessionId, expectedSequence }) => ({
      sessionId,
      expectedSequence,
    })),
    [{ sessionId: "session-1", expectedSequence: 2 }],
  );
  assert.equal(
    service.sessions.find(({ sessionId }) => sessionId === "session-1")
      ?.status,
    "archived",
  );
  assert.equal(service.active?.session.sessionId, "session-2");
  assert.equal(service.active?.threadId, "thread-2");
  assert.equal(service.state, "ready");
});

test("WorkbenchSessionService keeps an active local draft when no Session remains", async () => {
  const onlySession = session("session-1", "thread-1");
  const fake = fakeApi({ sessions: [onlySession] });
  using service = new WorkbenchSessionService(fake.api);

  await service.initialize();
  await service.archiveSession("session-1");

  assert.equal(service.active, undefined);
  assert.equal(service.drafts.length, 1);
  assert.equal(service.activeDraft?.title, "New Chat");
  assert.equal(fake.createSessionRequests.length, 0);
  assert.equal(fake.createThreadRequests.length, 0);
});

test("WorkbenchSessionService selects another draft and replaces the last discarded draft", async () => {
  const fake = fakeApi();
  using service = new WorkbenchSessionService(fake.api);

  await service.initialize();
  const initialDraft = service.activeDraft;
  assert.ok(initialDraft);
  const nextDraft = service.createDraft();

  service.discardDraft(nextDraft.draftId);
  assert.equal(service.activeDraft?.draftId, initialDraft.draftId);

  service.discardDraft(initialDraft.draftId);
  assert.equal(service.drafts.length, 1);
  assert.notEqual(service.activeDraft?.draftId, initialDraft.draftId);
  assert.equal(service.activeDraft?.title, "New Chat");
  assert.equal(fake.createSessionRequests.length, 0);
  assert.equal(fake.createThreadRequests.length, 0);
});

test("WorkbenchSessionService changes the model only for the selected Session", async () => {
  const fake = fakeApi({
    sessions: [
      session("session-1", "thread-1"),
      session("session-2", "thread-2"),
    ],
  });
  using service = new WorkbenchSessionService(fake.api);
  await service.initialize();
  const model: ModelRef = { provider: "openai", model: "gpt-session" };

  await service.setModel("session-1", model);

  assert.deepEqual(fake.setModelRequests.map(({ sessionId, expectedSequence, model }) => ({
    sessionId,
    expectedSequence,
    model,
  })), [{ sessionId: "session-1", expectedSequence: 2, model }]);
  assert.deepEqual(service.sessions.find(({ sessionId }) => sessionId === "session-1")?.model, model);
  assert.equal(service.sessions.find(({ sessionId }) => sessionId === "session-2")?.model, undefined);
  assert.deepEqual(service.active?.session.model, model);
});

test("ChatPaneModel layers transient deltas over canonical Thread state", async () => {
  const activeSession = session("session-1", "thread-1");
  let currentThread = thread();
  const fake = fakeApi({
    sessions: [activeSession],
    thread: () => currentThread,
  });
  using sessions = new WorkbenchSessionService(fake.api);
  using model = new ChatPaneModel(fake.api, {
    kind: "session",
    active: {
      session: activeSession,
      threadId: "thread-1",
    },
  }, sessions);

  await model.initialize();
  fake.emit({
    method: "thread/update",
    params: {
      sessionId: "session-1",
      threadId: "thread-1",
      durableSequence: 1,
      streamCursor: {
        streamInstanceId: "stream-1",
        sequence: 1,
      },
      update: {
        type: "itemStarted",
        turnId: "turn-1",
        item: {
          type: "agentMessage",
          itemId: "item-1",
          turnId: "turn-1",
          text: "",
        },
      },
    },
  });
  fake.emit({
    method: "thread/update",
    params: {
      sessionId: "session-1",
      threadId: "thread-1",
      durableSequence: 1,
      streamCursor: {
        streamInstanceId: "stream-1",
        sequence: 2,
      },
      update: {
        type: "itemDelta",
        turnId: "turn-1",
        itemId: "item-1",
        delta: { type: "agentMessage", text: "Hello" },
      },
    },
  });

  assert.deepEqual(model.items.map((item) => item.text), ["Hello"]);
  assert.equal(model.items[0].transient, true);

  currentThread = thread("Hello");
  fake.emit({
    method: "thread/update",
    params: {
      sessionId: "session-1",
      threadId: "thread-1",
      durableSequence: 4,
      update: {
        type: "committed",
        event: {
          type: "itemCompleted",
          threadId: "thread-1",
          turnId: "turn-1",
          item: currentThread.turns[0].items[0],
        },
      },
    },
  });
  await nextTask();

  assert.deepEqual(model.items.map((item) => item.text), ["Hello"]);
  assert.equal(model.items[0].transient, false);
  assert.equal(model.thread?.sequence, 4);
});

interface FakeOptions {
  readonly sessions?: readonly Session[];
  readonly createSession?: Session;
  readonly createSessionError?: Error;
  readonly createThread?: {
    readonly session: Session;
    readonly threadId: string;
  };
  readonly thread?: () => Thread;
}

function fakeApi(options: FakeOptions = {}): {
  readonly api: ZetaRendererApi;
  readonly archiveRequests: readonly SessionCommandParams[];
  readonly createSessionRequests: readonly SessionCreateParams[];
  readonly createThreadRequests: readonly SessionThreadCreateParams[];
  readonly setModelRequests: readonly SessionModelSetParams[];
  readonly turnStartRequests: readonly TurnStartParams[];
  readonly emit: (notification: ServerNotification) => void;
} {
  const listeners = new Set<(notification: ServerNotification) => void>();
  const archiveRequests: SessionCommandParams[] = [];
  const createSessionRequests: SessionCreateParams[] = [];
  const createThreadRequests: SessionThreadCreateParams[] = [];
  const setModelRequests: SessionModelSetParams[] = [];
  const turnStartRequests: TurnStartParams[] = [];
  const currentThread = () => options.thread?.() ?? thread();
  const api = {
    appServer: {
      getConnectionState: async () => "ready" as const,
      getSlashCommands: async () => [],
      onConnectionState: () => ({ dispose() {} }),
    },
    session: {
      list: async () => ({ sessions: [...(options.sessions ?? [])] }),
      create: async (params: SessionCreateParams) => {
        createSessionRequests.push(params);
        if (options.createSessionError) throw options.createSessionError;
        return { session: options.createSession ?? session("created") };
      },
      createThread: async (params: SessionThreadCreateParams) => {
        createThreadRequests.push(params);
        return options.createThread ?? {
          session: session("created", "created-thread"),
          threadId: "created-thread",
        };
      },
      archive: async (params: SessionCommandParams) => {
        archiveRequests.push(params);
        const archived = options.sessions?.find(
          ({ sessionId }) => sessionId === params.sessionId,
        ) ?? session(params.sessionId);
        return {
          session: {
            ...archived,
            status: "archived" as const,
            sequence: archived.sequence + 1,
          },
        };
      },
      setModel: async (params: SessionModelSetParams) => {
        setModelRequests.push(params);
        const current = options.sessions?.find(({ sessionId }) => sessionId === params.sessionId) ?? session(params.sessionId);
        return { session: { ...current, model: params.model, sequence: current.sequence + 1 } };
      },
    },
    model: {
      list: async () => ({ models: [] }),
    },
    thread: {
      read: async () => ({ thread: currentThread() }),
      subscribe: async () => ({
        thread: currentThread(),
        updates: [],
      }),
      unsubscribe: async () => undefined,
    },
    turn: {
      start: async (params: TurnStartParams) => {
        turnStartRequests.push(params);
        return { turnId: "turn-started", sequence: 2 };
      },
      interrupt: async () => ({ sequence: 3 }),
      resolveInteraction: async () => ({ sequence: 3 }),
    },
    events: {
      subscribe: (next: (notification: ServerNotification) => void) => {
        listeners.add(next);
        return { dispose: () => { listeners.delete(next); } };
      },
    },
  } as unknown as ZetaRendererApi;
  return {
    api,
    archiveRequests,
    createSessionRequests,
    createThreadRequests,
    setModelRequests,
    turnStartRequests,
    emit: (notification) => {
      for (const listener of listeners) listener(notification);
    },
  };
}

function session(
  id: string,
  threadId?: string,
  title = `Session ${id}`,
): Session {
  return {
    sessionId: id,
    title,
    status: "active",
    sequence: threadId ? 2 : 1,
    threads: threadId
      ? [{ threadId, origin: { type: "root" }, status: "active" }]
      : [],
  };
}

function thread(agentText?: string): Thread {
  return {
    sessionId: "session-1",
    threadId: "thread-1",
    title: "Main",
    status: "active",
    sequence: agentText ? 4 : 1,
    turns: agentText
      ? [{
        turnId: "turn-1",
        status: "completed",
        items: [{
          type: "agentMessage",
          itemId: "item-1",
          turnId: "turn-1",
          text: agentText,
        }],
      }]
      : [],
  };
}

function typeAlphaText(targetWindow: typeof browserEnvironment.window, input: HTMLTextAreaElement, text: string): void {
  input.dispatchEvent(new targetWindow.InputEvent("beforeinput", {
    bubbles: true,
    cancelable: true,
    data: text,
    inputType: "insertText",
  }));
}

function nextTask(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, 0));
}

async function waitFor(predicate: () => boolean): Promise<void> {
  for (let attempt = 0; attempt < 30; attempt += 1) {
    if (predicate()) return;
    await nextTask();
  }
  assert.fail("Timed out waiting for Chat view state");
}
