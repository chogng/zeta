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
Object.defineProperty(globalThis, "window", {
  configurable: true,
  value: browserEnvironment.window,
});
const { registerChatViews } = await import(
  "../src/zeta/workbench/contrib/chat/browser/chat.contribution.js"
);
test.after(() => {
  browserEnvironment.window.close();
  Reflect.deleteProperty(globalThis, "window");
});

test("Chat contribution owns the default Auxiliary Bar Composite", () => {
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
