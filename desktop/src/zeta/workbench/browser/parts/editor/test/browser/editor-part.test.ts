import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import type {
  IDimension,
} from "../../../../../../base/browser/geometry.js";
import { Emitter } from "../../../../../../base/common/event.js";
import {
  Keybinding,
  logicalKey,
  type ResolvedKeybinding,
  resolveKeybinding,
} from "../../../../../../base/common/keybindings.js";
import { DisposableOwner } from "../../../../../../base/common/lifecycle.js";
import { URI } from "../../../../../../base/common/uri.js";
import type {
  CommandId,
} from "../../../../../../platform/commands/common/commands.js";
import type {
  Context,
} from "../../../../../../platform/contextkey/common/contextkey.js";
import type {
  IKeybindingService,
} from "../../../../../../platform/keybinding/common/keybinding.js";
import type {
  EditorInput,
} from "../../../../../../workbench/browser/parts/editor/editorInput.js";
import {
  EditorPaneMatch,
  EditorPaneVisibility,
  type IEditorPane,
  type IEditorPaneDescriptor,
} from "../../../../../../workbench/browser/parts/editor/editorPane.js";
import {
  EditorPaneRegistry,
} from "../../../../../../workbench/browser/parts/editor/editorRegistry.js";

const browserEnvironment = new JSDOM("<!doctype html><body></body>");
for (const [name, value] of Object.entries({
  window: browserEnvironment.window,
  document: browserEnvironment.window.document,
  Node: browserEnvironment.window.Node,
  Element: browserEnvironment.window.Element,
  HTMLElement: browserEnvironment.window.HTMLElement,
})) {
  Object.defineProperty(globalThis, name, {
    configurable: true,
    value,
  });
}

const {
  EditorOpenSupersededError,
  EditorPart,
  IEditorPart,
} = await import(
  "../../../../../../workbench/browser/parts/editor/editorPart.js"
);
const {
  EditorGroupWatermarkEntries,
} = await import(
  "../../../../../../workbench/browser/parts/editor/editorGroupWatermark.js"
);
const { SplitEditorHorizontalCommandId } = await import(
  "../../../../../../workbench/browser/parts/editor/editorActions.js"
);
await import(
  "../../../../../../workbench/contrib/preferences/browser/preferences.contribution.js"
);

test.after(() => browserEnvironment.window.close());

test("editor registry resolves defaults and explicit Open With choices", () => {
  const registry = new EditorPaneRegistry();
  const monaco = descriptor(
    "zeta.editor.monaco",
    ".ts",
    () => new TestEditorPane("zeta.editor.monaco"),
  );
  const prosemirror = descriptor(
    "zeta.editor.prosemirror",
    ".md",
    () => new TestEditorPane("zeta.editor.prosemirror"),
  );
  const monacoRegistration = registry.register(monaco);
  const prosemirrorRegistration = registry.register(prosemirror);

  const typescript = input("C:\\project\\main.ts");
  const markdown = input("C:\\project\\paper.md");
  assert.equal(registry.resolve(typescript), monaco);
  assert.equal(registry.resolve(markdown), prosemirror);
  assert.deepEqual(registry.getEditors(markdown), [
    prosemirror,
    monaco,
  ]);
  assert.equal(
    registry.resolve(markdown, {
      preferredEditorId: "zeta.editor.monaco",
    }),
    monaco,
  );
  assert.throws(
    () => registry.resolve(markdown, {
      preferredEditorId: "zeta.editor.unknown",
    }),
    /Unknown editor pane/,
  );
  assert.throws(
    () => registry.register(monaco),
    /already registered/,
  );

  prosemirrorRegistration.dispose();
  assert.equal(registry.resolve(markdown), monaco);
  monacoRegistration.dispose();
  assert.throws(() => registry.resolve(markdown), /No editor can open/);
});

test("EditorPart passes the Workbench text-file service to pane factories", async () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const registry = new EditorPaneRegistry();
  const textFileService = {
    onDidChangeFiles: () => ({
      dispose() {},
      [Symbol.dispose]() {},
    }),
    resolve: async () => {
      throw new Error("not used");
    },
    save: async () => {
      throw new Error("not used");
    },
  };
  let observedTextFileService: unknown;
  registry.register({
    id: "zeta.editor.text-service-test",
    name: "Text Service Test",
    canOpen: () => EditorPaneMatch.Default,
    create: options => {
      observedTextFileService = options.textFileService;
      return new TestEditorPane("zeta.editor.text-service-test");
    },
  });
  const editor = new EditorPart(dom.window.document, {
    registry,
    textFileService,
  });

  await editor.openEditor(input("C:\\project\\main.ts"));

  assert.equal(observedTextFileService, textFileService);
  editor.dispose();
  dom.window.close();
});

test("EditorPart shows command shortcuts until an editor opens", async () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const registry = new EditorPaneRegistry();
  registry.register(descriptor(
    "zeta.editor.monaco",
    ".ts",
    () => new TestEditorPane("zeta.editor.monaco"),
  ));
  const keybindings = new TestKeybindingService();
  keybindings.set(
    "test.openEditor",
    Keybinding.single(logicalKey("o", { primaryKey: true })),
  );
  const entry = EditorGroupWatermarkEntries.register({
    id: "test.openEditor",
    label: "Open Editor",
    command: "test.openEditor",
  });
  const editor = new EditorPart(dom.window.document, {
    keybindingService: keybindings,
    registry,
  });
  dom.window.document.body.append(editor.element);

  assert.match(
    editor.element.textContent ?? "",
    /Open Editor.*Ctrl\+O/,
  );
  await editor.openEditor(input("C:\\project\\main.ts"));
  assert.equal(
    editor.element.querySelector<HTMLElement>(
      ".zeta-editor-group-watermark",
    )?.hidden,
    true,
  );

  editor.dispose();
  entry.dispose();
  keybindings.dispose();
  dom.window.close();
});

test("EditorPart retains tabs and switches loaded panes", async () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const registry = new EditorPaneRegistry();
  const panes: TestEditorPane[] = [];
  registry.register(descriptor(
    "zeta.editor.monaco",
    ".ts",
    () => trackPane(panes, "zeta.editor.monaco"),
  ));
  registry.register(descriptor(
    "zeta.editor.prosemirror",
    ".md",
    () => trackPane(panes, "zeta.editor.prosemirror"),
  ));
  const editor = new EditorPart(dom.window.document, { registry });
  dom.window.document.body.append(editor.element);

  const typescript = input("C:\\project\\main.ts");
  const monaco = await editor.openEditor(typescript);
  assert.equal(editor.groups.length, 1);
  assert.equal(editor.activeGroup, editor.groups[0]);
  assert.equal(editor.activePane, monaco);
  assert.equal(editor.activeInput, typescript);
  assert.deepEqual(editor.activeGroup.inputs, [typescript]);
  assert.equal(
    editor.element.querySelector(
      ".zeta-editor-pane-host:not([hidden])",
    )?.textContent,
    "zeta.editor.monaco",
  );
  assert.deepEqual(panes[0]?.visibilities, [
    EditorPaneVisibility.Hidden,
    EditorPaneVisibility.Visible,
  ]);
  const titleControl = editor.element.querySelector(
    ".zeta-editor-title-control",
  );
  const tablist = titleControl?.querySelector(
    ".zeta-editor-tabs-control .zeta-action-bar",
  );
  const toolbar = titleControl?.querySelector(
    ".zeta-editor-title-actions > .zeta-action-bar",
  );
  assert.equal(tablist?.getAttribute("role"), "tablist");
  assert.equal(toolbar?.getAttribute("role"), "toolbar");
  assert.equal(toolbar?.classList.contains("zeta-toolbar"), true);
  assert.equal(
    titleControl?.querySelector(
      ".zeta-editor-tabs-control .zeta-scrollable-element",
    )?.getAttribute("data-scroll-direction"),
    "horizontal",
  );
  assert.equal(
    tablist?.closest(".zeta-editor-tabs-control")?.nextElementSibling,
    toolbar?.parentElement,
  );
  const firstTab = tablist?.querySelector<HTMLElement>("[role='tab']");
  assert.equal(firstTab?.textContent, "main.ts");
  assert.equal(firstTab?.getAttribute("aria-selected"), "true");
  const firstPanelId = firstTab?.getAttribute("aria-controls");
  assert.ok(firstPanelId);
  assert.equal(
    editor.element.querySelector(`#${firstPanelId}`)?.getAttribute("role"),
    "tabpanel",
  );

  editor.layout({ width: 800, height: 600 });
  assert.deepEqual(panes[0]?.dimension, { width: 800, height: 565 });
  editor.focus();
  assert.equal(panes[0]?.focusCount, 1);

  const markdown = input("C:\\project\\paper.md");
  const prosemirror = await editor.openEditor(markdown);
  assert.equal(editor.activePane, prosemirror);
  assert.equal(editor.activeInput, markdown);
  assert.deepEqual(editor.activeGroup.inputs, [typescript, markdown]);
  assert.equal(
    editor.element.querySelector(
      ".zeta-editor-pane-host:not([hidden])",
    )?.textContent,
    "zeta.editor.prosemirror",
  );
  assert.equal(panes[0]?.disposed, false);
  assert.deepEqual(panes[0]?.visibilities.slice(-1), [
    EditorPaneVisibility.Hidden,
  ]);
  assert.deepEqual(panes[1]?.dimension, { width: 800, height: 565 });
  const tabs = editor.element.querySelectorAll<HTMLElement>("[role='tab']");
  assert.equal(tabs.length, 2);
  assert.deepEqual(
    [...tabs].map((tab) => tab.getAttribute("aria-selected")),
    ["false", "true"],
  );

  tabs[0]?.click();
  assert.equal(editor.activeInput, typescript);
  assert.equal(editor.activePane, monaco);
  assert.equal(panes[0]?.focusCount, 2);
  assert.deepEqual(
    [...editor.element.querySelectorAll<HTMLElement>("[role='tab']")]
      .map((tab) => tab.getAttribute("aria-selected")),
    ["true", "false"],
  );
  editor.element.querySelector<HTMLButtonElement>(
    ".zeta-editor-tabs-control .zeta-tab-actions button",
  )?.click();
  assert.equal(panes[0]?.disposed, true);
  assert.equal(editor.activeInput, markdown);
  assert.equal(editor.activePane, prosemirror);
  assert.deepEqual(editor.activeGroup.inputs, [markdown]);
  assert.equal(
    editor.element.querySelectorAll("[role='tab']").length,
    1,
  );

  const content = dom.window.document.createElement("div");
  content.textContent = "Welcome";
  editor.setContent(content);
  assert.equal(editor.activePane, undefined);
  assert.equal(editor.activeInput, undefined);
  assert.equal(panes[1]?.disposed, true);
  assert.equal(editor.element.textContent, "Welcome");

  editor.dispose();
  dom.window.close();
});

test("Editor title toolbar splits the active group and owns More Actions", async () => {
  const [
    { MenuService },
    { ContextKeyService },
    { ServiceCollection },
    { CommandService },
  ] = await Promise.all([
    import("../../../../../../platform/actions/common/menuService.js"),
    import("../../../../../../platform/contextkey/common/contextkey.js"),
    import("../../../../../../platform/instantiation/common/instantiation.js"),
    import("../../../../../../workbench/services/commands/common/commandService.js"),
  ]);
  const dom = new JSDOM("<!doctype html><body></body>");
  const registry = new EditorPaneRegistry();
  const panes: TestEditorPane[] = [];
  registry.register(descriptor(
    "zeta.editor.monaco",
    ".ts",
    () => trackPane(panes, "zeta.editor.monaco"),
  ));
  const services = new ServiceCollection();
  using contextKeys = new ContextKeyService();
  using commands = new CommandService(services);
  const menus = new MenuService(commands, contextKeys);
  const editor = new EditorPart(dom.window.document, {
    registry,
    titleActions: {
      menuService: menus,
      contextMenuProvider: {
        showContextMenu() {},
      },
    },
  });
  services.set(IEditorPart, editor);
  dom.window.document.body.append(editor.element);
  const activeInput = input("C:\\project\\main.ts");
  await editor.openEditor(activeInput);
  editor.layout({ width: 800, height: 600 });

  const toolbar = editor.element.querySelector(
    ".zeta-editor-title-actions > .zeta-toolbar",
  );
  assert.deepEqual(
    [...toolbar?.querySelectorAll<HTMLElement>("[data-action-id]") ?? []]
      .map((item) => item.dataset.actionId),
    [
      SplitEditorHorizontalCommandId,
      "zeta.toolbar.moreActions",
    ],
  );
  assert.deepEqual(
    [...toolbar?.querySelectorAll<HTMLButtonElement>("button") ?? []]
      .map((button) => button.title),
    ["Split Editor Horizontal", "More Actions"],
  );

  toolbar?.querySelector<HTMLButtonElement>(
    `[data-action-id="${SplitEditorHorizontalCommandId}"] button`,
  )?.click();
  await nextTask();

  assert.equal(editor.groups.length, 2);
  assert.equal(editor.activeGroup, editor.groups[1]);
  assert.deepEqual(
    editor.groups.map((group) => group.inputs),
    [[activeInput], [activeInput]],
  );
  assert.equal(
    editor.element.querySelectorAll(
      ":scope .zeta-split-view > .zeta-split-view-pane",
    ).length,
    2,
  );
  assert.equal(
    editor.element.querySelectorAll(
      ":scope .zeta-split-view > .zeta-sash",
    ).length,
    1,
  );
  assert.deepEqual(
    panes.map((pane) => pane.dimension),
    [
      { width: 400, height: 565 },
      { width: 400, height: 565 },
    ],
  );

  editor.dispose();
  dom.window.close();
});

test("EditorPart retains the active pane when a replacement fails", async () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const registry = new EditorPaneRegistry();
  const panes: TestEditorPane[] = [];
  registry.register(descriptor(
    "zeta.editor.working",
    ".ok",
    () => trackPane(panes, "zeta.editor.working"),
  ));
  registry.register(descriptor(
    "zeta.editor.failing",
    ".bad",
    () => {
      const pane = trackPane(panes, "zeta.editor.failing");
      pane.inputError = new Error("Unable to load input");
      return pane;
    },
  ));
  const editor = new EditorPart(dom.window.document, { registry });
  dom.window.document.body.append(editor.element);
  const workingInput = input("C:\\project\\document.ok");
  const workingPane = await editor.openEditor(workingInput);

  await assert.rejects(
    editor.openEditor(input("C:\\project\\document.bad")),
    /Unable to load input/,
  );
  assert.equal(editor.activePane, workingPane);
  assert.equal(editor.activeInput, workingInput);
  assert.equal(panes[1]?.disposed, true);

  editor.dispose();
  dom.window.close();
});

test("EditorPart rejects an open superseded by ordinary content", async () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const registry = new EditorPaneRegistry();
  const pending = deferred<void>();
  let slowPane: TestEditorPane | undefined;
  registry.register(descriptor(
    "zeta.editor.slow",
    ".slow",
    () => {
      const pane = new TestEditorPane("zeta.editor.slow");
      pane.inputPromise = pending.promise;
      slowPane = pane;
      return pane;
    },
  ));
  const editor = new EditorPart(dom.window.document, { registry });
  dom.window.document.body.append(editor.element);
  const opening = editor.openEditor(input("C:\\project\\document.slow"));
  const content = dom.window.document.createElement("div");
  content.textContent = "Replacement";
  editor.setContent(content);
  assert.equal(slowPane?.inputSignal?.aborted, true);
  pending.resolve(undefined);

  await assert.rejects(
    opening,
    EditorOpenSupersededError,
  );
  assert.equal(editor.activePane, undefined);
  assert.equal(editor.element.textContent, "Replacement");

  editor.dispose();
  dom.window.close();
});

class TestEditorPane extends DisposableOwner implements IEditorPane {
  readonly visibilities: EditorPaneVisibility[] = [];
  inputError: Error | undefined;
  inputPromise: Promise<void> | undefined;
  inputSignal: AbortSignal | undefined;
  dimension: IDimension | undefined;
  focusCount = 0;
  disposed = false;

  constructor(readonly id: string) {
    super();
    this.defer(() => {
      this.disposed = true;
    });
  }

  create(parent: HTMLElement): void {
    const element = parent.ownerDocument.createElement("div");
    element.textContent = this.id;
    parent.append(element);
  }

  async setInput(
    _input: EditorInput,
    signal: AbortSignal,
  ): Promise<void> {
    this.inputSignal = signal;
    if (this.inputError) throw this.inputError;
    await this.inputPromise;
  }

  clearInput(): void {}

  layout(dimension: IDimension): void {
    this.dimension = {
      width: dimension.width,
      height: dimension.height,
    };
  }

  setVisible(visibility: EditorPaneVisibility): void {
    this.visibilities.push(visibility);
  }

  focus(): void {
    this.focusCount += 1;
  }
}

function nextTask(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, 0));
}

class TestKeybindingService implements IKeybindingService {
  private readonly _onDidUpdateKeybindings = new Emitter<void>();
  private readonly bindings = new Map<CommandId, ResolvedKeybinding>();

  readonly inChordMode = false;
  readonly onDidUpdateKeybindings = this._onDidUpdateKeybindings.event;

  set(command: CommandId, keybinding: Keybinding): void {
    this.bindings.set(command, resolveKeybinding(keybinding));
    this._onDidUpdateKeybindings.fire();
  }

  resolveKeybinding(keybinding: Keybinding): ResolvedKeybinding {
    return resolveKeybinding(keybinding);
  }

  resolveUserBinding(_userBinding: string): ResolvedKeybinding | undefined {
    return undefined;
  }

  lookupKeybindings(
    command: CommandId,
    _context?: Context,
  ): readonly ResolvedKeybinding[] {
    const keybinding = this.lookupKeybinding(command);
    return keybinding ? [keybinding] : [];
  }

  lookupKeybinding(
    command: CommandId,
    _context?: Context,
  ): ResolvedKeybinding | undefined {
    return this.bindings.get(command);
  }

  dispose(): void {
    this._onDidUpdateKeybindings.dispose();
  }
}

function descriptor(
  id: string,
  defaultExtension: string,
  create: () => IEditorPane,
): IEditorPaneDescriptor {
  return {
    id,
    name: id,
    canOpen: (candidate) =>
      candidate.resource.path.endsWith(defaultExtension)
        ? EditorPaneMatch.Default
        : EditorPaneMatch.Optional,
    create,
  };
}

function input(path: string): EditorInput {
  return { resource: URI.file(path) };
}

function trackPane(
  panes: TestEditorPane[],
  id: string,
): TestEditorPane {
  const pane = new TestEditorPane(id);
  panes.push(pane);
  return pane;
}

function deferred<T>(): {
  readonly promise: Promise<T>;
  resolve(value: T): void;
} {
  let resolvePromise!: (value: T) => void;
  const promise = new Promise<T>((resolve) => {
    resolvePromise = resolve;
  });
  return { promise, resolve: resolvePromise };
}
