import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import type {
  WorkspaceSearchMatch,
} from "../generated/app-server/types.js";
import {
  BrowserWorkspaceSearchService,
  type IWorkspaceSearchApi,
} from "../src/zeta/platform/search/browser/searchService.js";
import type {
  IWorkspaceSearchQuery,
  IWorkspaceSearchService,
} from "../src/zeta/platform/search/common/search.js";

const matches: readonly WorkspaceSearchMatch[] = [
  {
    path: "src/main.ts",
    lineNumber: 4,
    preview: "const needle = true;",
    ranges: [{ start: 6, end: 12 }],
  },
  {
    path: "src/main.ts",
    lineNumber: 9,
    preview: "use(needle);",
    ranges: [{ start: 4, end: 10 }],
  },
];

test("BrowserWorkspaceSearchService pulls bounded batches and releases the job", async () => {
  const readCursors: number[] = [];
  let cancelCount = 0;
  const api: IWorkspaceSearchApi = {
    start: async (params) => {
      assert.equal(params.query, "needle");
      assert.equal(params.maxResults, 2_000);
      return { searchId: "search-1" };
    },
    read: async (params) => {
      readCursors.push(params.afterMatch);
      if (params.afterMatch === 0) {
        return {
          searchId: params.searchId,
          matches: [matches[0]],
          nextMatch: 1,
          completed: false,
          limitHit: false,
          error: null,
        };
      }
      return {
        searchId: params.searchId,
        matches: [matches[1]],
        nextMatch: 2,
        completed: true,
        limitHit: true,
        error: null,
      };
    },
    cancel: async () => {
      cancelCount += 1;
    },
  };
  const service = new BrowserWorkspaceSearchService(api);
  const progress: WorkspaceSearchMatch[] = [];

  const complete = await service.search(query(), {
    onProgress: (batch) => progress.push(...batch),
  });

  assert.deepEqual(readCursors, [0, 1]);
  assert.deepEqual(progress, matches);
  assert.deepEqual(complete, {
    resultCount: 2,
    limitHit: true,
    error: undefined,
  });
  assert.equal(cancelCount, 1);
});

test("SearchViewPane submits typed filters and groups highlighted matches", async () => {
  const browser = new JSDOM("<!doctype html><body></body>");
  const installedGlobals = installDomGlobals(browser);
  let submitted: IWorkspaceSearchQuery | undefined;
  const service: IWorkspaceSearchService = {
    search: async (searchQuery, options) => {
      submitted = searchQuery;
      options?.onProgress?.(matches);
      return {
        resultCount: matches.length,
        limitHit: false,
        error: undefined,
      };
    },
  };

  try {
    const { SearchViewPane } = await import(
      "../src/zeta/workbench/contrib/search/browser/searchViewPane.js"
    );
    using pane = new SearchViewPane(
      {
        id: "zeta.search",
        title: "Search",
        ownerDocument: browser.window.document,
      },
      service,
    );
    browser.window.document.body.append(pane.element);
    input(pane.element, "Search workspace").value = " needle ";
    input(pane.element, "Files to include").value = "src/**, docs/**";
    input(pane.element, "Files to exclude").value = "**/*.test.ts";
    const checkboxes = pane.element.querySelectorAll<HTMLInputElement>(
      'input[type="checkbox"]',
    );
    checkboxes[0].checked = true;
    checkboxes[1].checked = true;
    pane.element.querySelector("form")?.dispatchEvent(
      new browser.window.Event("submit", {
        bubbles: true,
        cancelable: true,
      }),
    );

    await waitFor(() =>
      pane.element.querySelector(".zeta-search-status")?.textContent ===
        "2 results"
    );
    assert.deepEqual(submitted, {
      text: "needle",
      patternKind: "regex",
      caseSensitivity: "sensitive",
      includePatterns: ["src/**", "docs/**"],
      excludePatterns: ["**/*.test.ts"],
    });
    assert.equal(
      pane.element.querySelector(".zeta-search-file-path")?.textContent,
      "src/main.ts",
    );
    assert.equal(
      pane.element.querySelector(".zeta-search-file-count")?.textContent,
      "2",
    );
    assert.deepEqual(
      [...pane.element.querySelectorAll("mark")].map(
        (element) => element.textContent,
      ),
      ["needle", "needle"],
    );
  } finally {
    browser.window.close();
    for (const name of installedGlobals) {
      Reflect.deleteProperty(globalThis, name);
    }
  }
});

function query(): IWorkspaceSearchQuery {
  return {
    text: "needle",
    patternKind: "literal",
    caseSensitivity: "smart",
    includePatterns: [],
    excludePatterns: [],
  };
}

function input(container: Element, label: string): HTMLInputElement {
  const element = container.querySelector<HTMLInputElement>(
    `input[aria-label="${label}"]`,
  );
  assert.ok(element);
  return element;
}

async function waitFor(
  condition: () => boolean,
  timeoutMillis = 1_000,
): Promise<void> {
  const deadline = Date.now() + timeoutMillis;
  while (!condition()) {
    if (Date.now() >= deadline) {
      throw new Error("Timed out waiting for SearchViewPane");
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
