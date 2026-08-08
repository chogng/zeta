import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { URI } from "../../../../../base/common/uri.js";
import { FileKind, type IFileService } from "../../../../../platform/files/common/files.js";
import { WorkspaceContextService } from "../../../../../workbench/services/workspaces/browser/workspaceContextService.js";
import type { IFileIconThemeService } from "../../../../../platform/theme/browser/fileIconThemeService.js";
import type { IHoverService, IManagedHover } from "../../../../../platform/hover/common/hoverService.js";
import type { EditorInput } from "../../../../../workbench/browser/parts/editor/editorInput.js";
import type { IEditorPart } from "../../../../../workbench/browser/parts/editor/editorPart.js";

test("ExplorerViewPane renders, expands, and opens workspace files", async () => {
  const browser = new JSDOM("<!doctype html><body></body>");
  const installedGlobals = installDomGlobals(browser);
  const root = URI.file("C:\\project");
  const nextRoot = URI.file("C:\\next-project");
  const directoryReads: string[] = [];
  let openedInput: EditorInput | undefined;
  const fileService: IFileService = {
    onDidChangeFiles: () => ({
      dispose() {},
      [Symbol.dispose]() {},
    }),
    stat: async (resource) => ({
      resource,
      kind: FileKind.Directory,
      sizeBytes: 0,
      readonly: false,
      modifiedAtMillis: undefined,
    }),
    readDirectory: async (resource) => {
      directoryReads.push(resource.toString());
      if (resource.toString() === root.toString()) {
        return [
          {
            resource: URI.file("C:\\project\\README.md"),
            name: "README.md",
            kind: FileKind.File,
          },
          {
            resource: URI.file("C:\\project\\src"),
            name: "src",
            kind: FileKind.Directory,
          },
        ];
      }
      if (resource.toString() === nextRoot.toString()) {
        return [{
          resource: URI.file("C:\\next-project\\next.txt"),
          name: "next.txt",
          kind: FileKind.File,
        }];
      }
      return [{
        resource: URI.file("C:\\project\\src\\main.ts"),
        name: "main.ts",
        kind: FileKind.File,
      }];
    },
    readFile: async (_resource) => {
      throw new Error("Explorer must delegate file content resolution to the selected editor");
    },
    readFileBytes: async (_resource) => {
      throw new Error("Explorer must delegate file content resolution to the selected editor");
    },
    writeFile: async (_request) => {
      throw new Error("Explorer must delegate file writes to the selected editor");
    },
  };
  using workspaceContextService = new WorkspaceContextService({
    id: "workspace",
    uri: root,
  });
  const editorPart: IEditorPart = {
    element: browser.window.document.createElement("section"),
    groups: [],
    activeGroup: {} as never,
    activeInput: undefined,
    activePane: undefined,
    openEditor: async (input: EditorInput) => {
      openedInput = input;
      return {} as never;
    },
    activateEditor: () => {
      throw new Error("No active editor");
    },
    closeEditor() {},
    async saveActiveEditor() {},
    setContent() {},
    async splitActiveGroupHorizontal() {},
    layout() {},
    focus() {},
  };
  const fileIconThemeService: IFileIconThemeService = {
    onDidFileIconThemeChange: () => ({
      dispose() {},
      [Symbol.dispose]() {},
    }),
    renderFileIcon: (resource, container) => {
      container.classList.add("zeta-seti-file-icon");
      container.textContent = resource.path.endsWith(".ts") ? "T" : "F";
    },
  };
  const hoverService: IHoverService = {
    setupHover: () => testManagedHover(),
    showHover: () => testManagedHover(),
    hideHover() {},
  };

  try {
    const { ExplorerViewPane } = await import(
      "../../../../../workbench/contrib/files/browser/explorerViewPane.js"
    );
    const { EmptyView } = await import(
      "../../../../../workbench/contrib/files/browser/views/emptyView.js"
    );
    let folderOpens = 0;
    using emptyView = new EmptyView(
      {
        id: EmptyView.ID,
        title: EmptyView.TITLE,
        ownerDocument: browser.window.document,
      },
      {
        canOpenFolder: true,
        openFolder: async () => {
          folderOpens += 1;
        },
      },
    );
    assert.equal(emptyView.element.dataset.viewId, EmptyView.ID);
    assert.equal(
      emptyView.element.querySelector(
        ".zeta-empty-explorer-message",
      )?.textContent,
      "Open a folder to explore its files.",
    );
    const openFolderButton =
      emptyView.element.querySelector<HTMLButtonElement>(
        ".zeta-empty-explorer-open-folder",
      );
    assert.equal(openFolderButton?.textContent, "Open Folder");
    openFolderButton?.click();
    await waitFor(() => folderOpens === 1);
    assert.equal(openFolderButton?.disabled, false);

    using pane = new ExplorerViewPane(
      {
        id: "zeta.explorer",
        title: "Explorer",
        ownerDocument: browser.window.document,
      },
      fileService,
      workspaceContextService,
      editorPart,
      fileIconThemeService,
      hoverService,
    );
    browser.window.document.body.append(pane.element);
    assert.equal(
      pane.element.querySelector(".zeta-explorer-status")?.textContent,
      "Loading files…",
    );

    await waitFor(() => pane.element.querySelectorAll(
      ".zeta-tree-row",
    ).length === 2);
    assert.equal(
      pane.element.querySelector(".zeta-pane-view-header-title")?.textContent,
      "project",
    );
    assert.equal(
      pane.element.querySelector(".zeta-pane-view-header")?.classList.contains("zeta-explorer-title"),
      true,
    );
    assert.deepEqual(rowLabels(pane.element), ["src", "README.md"]);
    assert.equal(
      pane.element.querySelector(
        ".zeta-explorer > .zeta-scrollable-element",
      )?.getAttribute("data-scroll-direction"),
      "vertical",
    );
    assert.equal(
      pane.element.querySelectorAll(
        ".zeta-tree-twistie .zeta-icon",
      ).length,
      1,
    );
    assert.ok(pane.element.querySelector(".zeta-tree-indent-guides-always"));
    assert.equal(
      pane.element.querySelectorAll(".zeta-seti-file-icon").length,
      1,
    );

    const sourceFolder = [...pane.element.querySelectorAll<HTMLButtonElement>(
      ".zeta-tree-row",
    )].find((row) => rowLabel(row) === "src");
    assert.ok(sourceFolder);
    sourceFolder.click();

    await waitFor(() => rowLabels(pane.element).includes("main.ts"));
    assert.deepEqual(rowLabels(pane.element), [
      "src",
      "main.ts",
      "README.md",
    ]);
    assert.equal(
      pane.element.querySelectorAll(".zeta-seti-file-icon").length,
      2,
    );
    assert.deepEqual(directoryReads, [
      root.toString(),
      URI.file("C:\\project\\src").toString(),
    ]);

    const readme = [...pane.element.querySelectorAll<HTMLButtonElement>(
      ".zeta-tree-row",
    )].find((row) => rowLabel(row) === "README.md");
    assert.ok(readme);
    readme.click();
    await waitFor(() => openedInput !== undefined);
    assert.equal(openedInput?.label, "README.md");
    assert.equal(openedInput?.initialText, undefined);
    assert.equal(
      openedInput?.resource.toString(),
      URI.file("C:\\project\\README.md").toString(),
    );

    workspaceContextService.updateWorkspace({
      id: "next-workspace",
      uri: nextRoot,
    });
    await waitFor(() =>
      pane.element.querySelector(".zeta-pane-view-header-title")?.textContent ===
        "next-project" &&
      rowLabels(pane.element).includes("next.txt")
    );
    assert.deepEqual(rowLabels(pane.element), ["next.txt"]);
  } finally {
    browser.window.close();
    for (const name of installedGlobals) {
      Reflect.deleteProperty(globalThis, name);
    }
  }
});

function testManagedHover(): IManagedHover {
  return {
    visible: false,
    show() {},
    hide() {},
    update() {},
    dispose() {},
    [Symbol.dispose]() {},
  };
}

function rowLabels(container: Element): readonly string[] {
  return [...container.querySelectorAll<HTMLElement>(
    ".zeta-tree-row",
  )].map(rowLabel);
}

function rowLabel(row: Element): string {
  return row.querySelector(".zeta-icon-label-text")?.textContent ?? "";
}

async function waitFor(
  condition: () => boolean,
  timeoutMillis = 1_000,
): Promise<void> {
  const deadline = Date.now() + timeoutMillis;
  while (!condition()) {
    if (Date.now() >= deadline) {
      throw new Error("Timed out waiting for ExplorerViewPane");
    }
    await new Promise((resolve) => setTimeout(resolve, 0));
  }
}

function installDomGlobals(browser: JSDOM): readonly string[] {
  const globals = {
    window: browser.window,
    document: browser.window.document,
    Node: browser.window.Node,
    Element: browser.window.Element,
    HTMLElement: browser.window.HTMLElement,
    Event: browser.window.Event,
    MouseEvent: browser.window.MouseEvent,
    KeyboardEvent: browser.window.KeyboardEvent,
    navigator: browser.window.navigator,
  };
  for (const [name, value] of Object.entries(globals)) {
    Object.defineProperty(globalThis, name, {
      configurable: true,
      value,
    });
  }
  return Object.keys(globals);
}
