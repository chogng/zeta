import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import type {
  ServerNotification,
  Session,
  Thread,
} from "../generated/app-server/types.js";
import type {
  ZetaRendererApi,
} from "../src/zeta/platform/app-server/common/renderer-api.js";
import { MenuService } from "../src/zeta/platform/actions/common/menuService.js";
import type { IContextMenuService } from "../src/zeta/platform/contextview/browser/contextMenu.js";
import { ServiceCollection } from "../src/zeta/platform/instantiation/common/instantiation.js";
import { CommandService } from "../src/zeta/workbench/services/commands/common/commandService.js";
import type {
  ViewPaneContainer,
} from "../src/zeta/workbench/browser/parts/views/viewPaneContainer.js";
import {
  ViewContainerLocation,
  WorkbenchViewRegistry,
} from "../src/zeta/workbench/common/views.js";
import {
  ChatViewModel,
} from "../src/zeta/workbench/contrib/chat/browser/chatViewModel.js";
import {
  CHAT_VIEW_CONTAINER_ID,
  CHAT_VIEW_ID,
  NEW_CHAT_COMMAND_ID,
} from "../src/zeta/workbench/contrib/chat/common/chat.js";
import {
  WorkbenchSessionService,
} from "../src/zeta/workbench/services/sessions/common/sessionService.js";
import {
  ViewsService,
} from "../src/zeta/workbench/services/views/browser/viewsService.js";
import {
  ContextKeyService,
} from "../src/zeta/platform/contextkey/common/contextkey.js";
import {
  ViewDescriptorService,
} from "../src/zeta/workbench/services/views/common/viewDescriptorService.js";

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
  "../src/zeta/workbench/contrib/chat/browser/chatViewPane.js"
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

test("Chat title separates Thread tabs from its action toolbar", async () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const api = fakeApi({
    sessions: [
      session("session-1", "thread-1"),
      session("session-2", "thread-2"),
    ],
  }).api;
  using sessions = new WorkbenchSessionService(api);
  using contextKeys = new ContextKeyService();
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
  );
  dom.window.document.body.append(pane.element);

  await sessions.initialize();
  await nextTask();

  const title = pane.element.querySelector(".zeta-chat-title-control");
  const tablist = title?.querySelector(
    ".zeta-chat-tabs-control .zeta-action-bar",
  );
  const toolbar = title?.querySelector(
    ".zeta-chat-title-actions > .zeta-action-bar",
  );
  assert.equal(tablist?.getAttribute("role"), "tablist");
  assert.equal(toolbar?.getAttribute("role"), "toolbar");
  assert.equal(
    tablist?.closest(".zeta-chat-tabs-control")?.nextElementSibling,
    toolbar?.parentElement,
  );
  assert.ok(toolbar?.querySelector(
    `[data-action-id="${NEW_CHAT_COMMAND_ID}"]`,
  ));
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

  tabs?.[1]?.click();
  assert.equal(sessions.active?.session.sessionId, "session-2");
  assert.equal(sessions.active?.threadId, "thread-2");
  assert.deepEqual(
    [...pane.element.querySelectorAll<HTMLElement>("[role='tab']")]
      .map((tab) => tab.getAttribute("aria-selected")),
    ["false", "true"],
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

test("ChatViewModel layers transient deltas over canonical Thread state", async () => {
  const activeSession = session("session-1", "thread-1");
  let currentThread = thread();
  const fake = fakeApi({
    sessions: [activeSession],
    thread: () => currentThread,
  });
  using sessions = new WorkbenchSessionService(fake.api);
  using model = new ChatViewModel(fake.api, sessions);

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
  readonly createThread?: {
    readonly session: Session;
    readonly threadId: string;
  };
  readonly thread?: () => Thread;
}

function fakeApi(options: FakeOptions = {}): {
  readonly api: ZetaRendererApi;
  readonly emit: (notification: ServerNotification) => void;
} {
  let listener: ((notification: ServerNotification) => void) | undefined;
  const currentThread = () => options.thread?.() ?? thread();
  const api = {
    appServer: {
      getConnectionState: async () => "ready" as const,
      onConnectionState: () => ({ dispose() {} }),
    },
    session: {
      list: async () => ({ sessions: [...(options.sessions ?? [])] }),
      create: async () => ({
        session: options.createSession ?? session("created"),
      }),
      createThread: async () =>
        options.createThread ?? {
          session: session("created", "created-thread"),
          threadId: "created-thread",
        },
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
      start: async () => ({ turnId: "turn-started", sequence: 2 }),
      interrupt: async () => ({ sequence: 3 }),
      resolveInteraction: async () => ({ sequence: 3 }),
    },
    events: {
      subscribe: (next: (notification: ServerNotification) => void) => {
        listener = next;
        return { dispose: () => { listener = undefined; } };
      },
    },
  } as unknown as ZetaRendererApi;
  return {
    api,
    emit: (notification) => listener?.(notification),
  };
}

function session(id: string, threadId?: string): Session {
  return {
    sessionId: id,
    title: `Session ${id}`,
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

function nextTask(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, 0));
}
