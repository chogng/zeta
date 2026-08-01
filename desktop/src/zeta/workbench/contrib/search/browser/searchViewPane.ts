import { addDisposableListener } from "../../../../base/browser/dom.js";
import type { IWorkspaceSearchQuery, IWorkspaceSearchService, WorkspaceSearchMatch, WorkspaceSearchMatchRange } from "../../../../platform/search/common/search.js";
import { ViewPane, type IViewPaneOptions } from "../../../browser/parts/views/viewPane.js";

interface SearchFileGroup {
  readonly matches: HTMLUListElement;
  readonly count: HTMLSpanElement;
  resultCount: number;
}

/** Workspace content-search form and incrementally populated result tree. */
export class SearchViewPane extends ViewPane {
  private readonly searchService: IWorkspaceSearchService;
  private readonly queryInput: HTMLInputElement;
  private readonly caseSensitiveInput: HTMLInputElement;
  private readonly regexInput: HTMLInputElement;
  private readonly includeInput: HTMLInputElement;
  private readonly excludeInput: HTMLInputElement;
  private readonly submitButton: HTMLButtonElement;
  private readonly statusElement: HTMLDivElement;
  private readonly resultsElement: HTMLUListElement;
  private readonly groups = new Map<string, SearchFileGroup>();
  private searchController: AbortController | undefined;
  private searchRevision = 0;
  private disposed = false;

  constructor(
    options: IViewPaneOptions,
    searchService: IWorkspaceSearchService,
  ) {
    super(options);
    this.searchService = searchService;
    this.contentElement.classList.add("zeta-search");
    const document = options.ownerDocument;
    const form = document.createElement("form");
    form.className = "zeta-search-form";
    this.queryInput = input(document, {
      className: "zeta-search-query",
      placeholder: "Search",
      ariaLabel: "Search workspace",
    });
    this.submitButton = document.createElement("button");
    this.submitButton.type = "submit";
    this.submitButton.className = "zeta-search-submit";
    this.submitButton.textContent = "Search";
    const toggles = document.createElement("div");
    toggles.className = "zeta-search-toggles";
    this.caseSensitiveInput = checkbox(
      document,
      toggles,
      "Match Case",
    );
    this.regexInput = checkbox(document, toggles, "Use Regex");
    const filters = document.createElement("div");
    filters.className = "zeta-search-filters";
    this.includeInput = input(document, {
      className: "zeta-search-filter",
      placeholder: "files to include (for example src/**)",
      ariaLabel: "Files to include",
    });
    this.excludeInput = input(document, {
      className: "zeta-search-filter",
      placeholder: "files to exclude",
      ariaLabel: "Files to exclude",
    });
    filters.append(this.includeInput, this.excludeInput);
    form.append(
      this.queryInput,
      this.submitButton,
      toggles,
      filters,
    );
    this.statusElement = document.createElement("div");
    this.statusElement.className = "zeta-search-status";
    this.statusElement.setAttribute("role", "status");
    this.statusElement.setAttribute("aria-live", "polite");
    this.statusElement.textContent = "Type a query and press Enter.";
    this.resultsElement = document.createElement("ul");
    this.resultsElement.className = "zeta-search-results";
    this.resultsElement.setAttribute("role", "tree");
    this.contentElement.append(
      form,
      this.statusElement,
      this.resultsElement,
    );
    this.own(addDisposableListener(form, "submit", (event) => {
      event.preventDefault();
      void this.startSearch();
    }));
    this.defer(() => {
      this.disposed = true;
      this.searchController?.abort();
      this.groups.clear();
    });
  }

  private async startSearch(): Promise<void> {
    const text = this.queryInput.value.trim();
    if (!text) {
      this.statusElement.textContent = "Enter text to search.";
      this.queryInput.focus();
      return;
    }
    this.searchController?.abort();
    const AbortControllerConstructor =
      this.element.ownerDocument.defaultView?.AbortController ??
      AbortController;
    const controller = new AbortControllerConstructor();
    this.searchController = controller;
    const revision = ++this.searchRevision;
    this.groups.clear();
    this.resultsElement.replaceChildren();
    this.submitButton.disabled = true;
    this.statusElement.textContent = "Searching workspace…";
    try {
      const complete = await this.searchService.search(
        this.query(text),
        {
          signal: controller.signal,
          onProgress: (matches) => {
            if (
              this.disposed ||
              revision !== this.searchRevision
            ) return;
            this.appendMatches(matches);
            this.statusElement.textContent =
              `${resultCount(this.groups)} results…`;
          },
        },
      );
      if (this.disposed || revision !== this.searchRevision) return;
      if (complete.error) {
        this.statusElement.textContent = complete.error;
      } else if (complete.resultCount === 0) {
        this.statusElement.textContent = "No results found.";
      } else {
        this.statusElement.textContent =
          `${complete.resultCount} results` +
          (complete.limitHit ? " (result limit reached)" : "");
      }
    } catch (error) {
      if (
        this.disposed ||
        revision !== this.searchRevision ||
        isAbortError(error)
      ) return;
      this.statusElement.textContent = error instanceof Error
        ? error.message
        : "Workspace search failed.";
    } finally {
      if (!this.disposed && revision === this.searchRevision) {
        this.submitButton.disabled = false;
        this.searchController = undefined;
      }
    }
  }

  private query(text: string): IWorkspaceSearchQuery {
    return {
      text,
      patternKind: this.regexInput.checked ? "regex" : "literal",
      caseSensitivity: this.caseSensitiveInput.checked
        ? "sensitive"
        : "smart",
      includePatterns: patterns(this.includeInput.value),
      excludePatterns: patterns(this.excludeInput.value),
    };
  }

  private appendMatches(matches: readonly WorkspaceSearchMatch[]): void {
    const document = this.element.ownerDocument;
    for (const match of matches) {
      let group = this.groups.get(match.path);
      if (!group) {
        const item = document.createElement("li");
        item.className = "zeta-search-file";
        item.setAttribute("role", "treeitem");
        item.setAttribute("aria-expanded", "true");
        const heading = document.createElement("div");
        heading.className = "zeta-search-file-heading";
        const path = document.createElement("span");
        path.className = "zeta-search-file-path";
        path.textContent = match.path;
        const count = document.createElement("span");
        count.className = "zeta-search-file-count";
        const resultList = document.createElement("ul");
        resultList.className = "zeta-search-file-matches";
        resultList.setAttribute("role", "group");
        heading.append(path, count);
        item.append(heading, resultList);
        this.resultsElement.append(item);
        group = {
          matches: resultList,
          count,
          resultCount: 0,
        };
        this.groups.set(match.path, group);
      }
      group.resultCount += 1;
      group.count.textContent = String(group.resultCount);
      group.matches.append(renderMatch(document, match));
    }
  }
}

function input(
  document: Document,
  options: {
    readonly className: string;
    readonly placeholder: string;
    readonly ariaLabel: string;
  },
): HTMLInputElement {
  const element = document.createElement("input");
  element.type = "text";
  element.className = options.className;
  element.placeholder = options.placeholder;
  element.setAttribute("aria-label", options.ariaLabel);
  element.autocomplete = "off";
  element.spellcheck = false;
  return element;
}

function checkbox(
  document: Document,
  parent: HTMLElement,
  text: string,
): HTMLInputElement {
  const label = document.createElement("label");
  label.className = "zeta-search-toggle";
  const element = document.createElement("input");
  element.type = "checkbox";
  label.append(element, document.createTextNode(text));
  parent.append(label);
  return element;
}

function patterns(value: string): readonly string[] {
  return value
    .split(",")
    .map((pattern) => pattern.trim())
    .filter((pattern) => pattern.length > 0);
}

function renderMatch(
  document: Document,
  match: WorkspaceSearchMatch,
): HTMLLIElement {
  const item = document.createElement("li");
  item.className = "zeta-search-match";
  item.setAttribute("role", "treeitem");
  const line = document.createElement("span");
  line.className = "zeta-search-line-number";
  line.textContent = String(match.lineNumber);
  const preview = document.createElement("code");
  preview.className = "zeta-search-preview";
  appendHighlightedPreview(
    document,
    preview,
    match.preview,
    match.ranges,
  );
  item.append(line, preview);
  return item;
}

function appendHighlightedPreview(
  document: Document,
  container: HTMLElement,
  text: string,
  ranges: readonly WorkspaceSearchMatchRange[],
): void {
  let offset = 0;
  for (const range of normalizedRanges(ranges, text.length)) {
    if (range.start > offset) {
      container.append(document.createTextNode(
        text.slice(offset, range.start),
      ));
    }
    const mark = document.createElement("mark");
    mark.textContent = text.slice(range.start, range.end);
    container.append(mark);
    offset = range.end;
  }
  if (offset < text.length) {
    container.append(document.createTextNode(text.slice(offset)));
  }
}

function normalizedRanges(
  ranges: readonly WorkspaceSearchMatchRange[],
  length: number,
): readonly WorkspaceSearchMatchRange[] {
  const normalized: WorkspaceSearchMatchRange[] = [];
  for (const range of [...ranges].sort((left, right) =>
    left.start - right.start || left.end - right.end
  )) {
    const start = Math.max(0, Math.min(length, range.start));
    const end = Math.max(start, Math.min(length, range.end));
    const previous = normalized.at(-1);
    if (previous && start <= previous.end) {
      previous.end = Math.max(previous.end, end);
    } else if (start !== end) {
      normalized.push({ start, end });
    }
  }
  return normalized;
}

function resultCount(groups: ReadonlyMap<string, SearchFileGroup>): number {
  let count = 0;
  for (const group of groups.values()) count += group.resultCount;
  return count;
}

function isAbortError(error: unknown): boolean {
  return error instanceof DOMException && error.name === "AbortError";
}
