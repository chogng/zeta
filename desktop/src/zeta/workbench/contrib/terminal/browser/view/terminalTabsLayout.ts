import { SplitView, type ISplitViewView } from "../../../../../base/browser/ui/splitview/splitview.js";
import { DisposableOwner } from "../../../../../base/common/lifecycle.js";

const TerminalTabsListSizes = {
  narrow: 46,
  wideMinimum: 80,
  default: 120,
  midpoint: 63,
  maximum: 500,
} as const;
const MIN_TERMINAL_WIDTH = 120;
const TERMINAL_VIEW_INDEX = 0;
const TABS_VIEW_INDEX = 1;

/** Owns the VS Code-style horizontal split between terminal widgets and their right-side instance list. */
export class TerminalTabsLayout extends DisposableOwner {
  readonly element: HTMLElement;
  readonly #splitView: SplitView;
  readonly #tabsElement: HTMLElement;
  #snapping = false;

  constructor(widgetsElement: HTMLElement, tabsElement: HTMLElement) {
    super();
    this.#tabsElement = tabsElement;
    this.#splitView = this.own(new SplitView("horizontal", widgetsElement.ownerDocument));
    this.element = this.#splitView.element;
    this.element.classList.add("zeta-terminal-tabs-layout");
    this.#splitView.addView(splitViewItem(widgetsElement, MIN_TERMINAL_WIDTH, Number.POSITIVE_INFINITY, "high"), { type: "distribute" });
    this.#splitView.addView(splitViewItem(tabsElement, TerminalTabsListSizes.narrow, TerminalTabsListSizes.maximum, "low"), TerminalTabsListSizes.default);
    this.element.querySelector<HTMLElement>(":scope > .zeta-sash")?.setAttribute("aria-label", "Resize terminal instance list");
    this.own(this.#splitView.onDidChangeViewSizes(() => this.#updateTabsWidth()));
    this.#updateTabsPresentation(TerminalTabsListSizes.default);
  }

  layout(width: number, height: number): void {
    this.#splitView.layout(width, height);
    this.#updateTabsWidth();
  }

  #updateTabsWidth(): void {
    if (this.#snapping) return;
    const width = this.#splitView.getViewSize(TABS_VIEW_INDEX);
    const snappedWidth = width < TerminalTabsListSizes.midpoint
      ? TerminalTabsListSizes.narrow
      : width < TerminalTabsListSizes.wideMinimum
        ? TerminalTabsListSizes.wideMinimum
        : width;
    if (snappedWidth !== width) {
      this.#snapping = true;
      this.#splitView.resizeView(TABS_VIEW_INDEX, snappedWidth);
      this.#snapping = false;
    }
    this.#updateTabsPresentation(snappedWidth);
  }

  #updateTabsPresentation(width: number): void {
    this.#tabsElement.classList.toggle("zeta-terminal-tabs-narrow", width < TerminalTabsListSizes.midpoint);
  }
}

function splitViewItem(element: HTMLElement, minimumSize: number, maximumSize: number, priority: "high" | "low"): ISplitViewView {
  return {
    element,
    minimumSize,
    maximumSize,
    priority,
    layout() {},
  };
}
