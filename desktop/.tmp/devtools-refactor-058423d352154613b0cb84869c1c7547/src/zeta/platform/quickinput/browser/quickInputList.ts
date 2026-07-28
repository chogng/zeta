import {
  List,
} from "../../../base/browser/ui/list/list.js";
import { setRole } from "../../../base/browser/ui/aria/aria.js";
import { Emitter, type Event } from "../../../base/common/event.js";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
import type { IQuickPickItem } from "../common/quickInput.js";

export interface QuickInputListActiveChangeEvent<TItem> {
  readonly item: TItem | undefined;
  readonly rowId: string | undefined;
}

/** Searchable single-selection list shared by browser Quick Inputs. */
export class QuickInputList<TItem extends IQuickPickItem>
  extends DisposableOwner {
  readonly element: HTMLDivElement;
  readonly #empty: HTMLDivElement;
  readonly #list: List<TItem>;
  readonly #onDidAccept = this.own(new Emitter<TItem>());
  readonly #onDidChangeActive =
    this.own(new Emitter<QuickInputListActiveChangeEvent<TItem>>());
  #items: readonly TItem[] = [];
  #visibleItems: readonly TItem[] = [];
  #query = "";

  readonly onDidAccept: Event<TItem> = this.#onDidAccept.event;
  readonly onDidChangeActive:
    Event<QuickInputListActiveChangeEvent<TItem>> =
      this.#onDidChangeActive.event;

  constructor(ownerDocument: Document) {
    super();
    this.element = ownerDocument.createElement("div");
    this.element.className = "zeta-quick-pick-list";
    this.defer(() => this.element.remove());

    this.#list = this.own(new List<TItem>({
      ownerDocument,
      ariaLabel: "Quick Pick results",
      renderItem: (item) => this.#renderItem(item),
    }));
    this.#list.element.classList.add("zeta-quick-pick-list-items");
    this.#empty = ownerDocument.createElement("div");
    this.#empty.className = "zeta-quick-pick-empty";
    setRole(this.#empty, "status");
    this.#empty.textContent = "No matching results";
    this.#empty.hidden = true;
    this.element.append(this.#list.element, this.#empty);

    this.own(this.#list.onDidAccept(({ item }) => {
      this.#onDidAccept.fire(item);
    }));
    this.own(this.#list.onDidChangeActive(({ item, rowId }) => {
      this.#onDidChangeActive.fire({ item, rowId });
    }));
  }

  get listId(): string {
    return this.#list.element.id;
  }

  get items(): readonly TItem[] {
    return this.#items;
  }

  set items(items: readonly TItem[]) {
    this.#items = [...items];
    this.#render();
  }

  get visibleItems(): readonly TItem[] {
    return this.#visibleItems;
  }

  get activeItem(): TItem | undefined {
    return this.#list.activeItem;
  }

  filter(query: string): void {
    if (this.#query === query) return;
    this.#query = query;
    this.#render();
  }

  focusNext(): void {
    this.#list.focusNext();
  }

  focusPrevious(): void {
    this.#list.focusPrevious();
  }

  acceptActive(): void {
    this.#list.acceptActive();
  }

  #render(): void {
    this.#visibleItems = filterQuickPickItems(
      this.#items,
      this.#query,
    );
    this.#list.items = this.#visibleItems;
    const empty = this.#visibleItems.length === 0;
    this.#list.element.hidden = empty;
    this.#empty.hidden = !empty;
  }

  #renderItem(item: TItem): HTMLDivElement {
    const ownerDocument = this.element.ownerDocument;
    const content = ownerDocument.createElement("div");
    content.className = "zeta-quick-pick-row-content";
    const text = ownerDocument.createElement("span");
    text.className = "zeta-quick-pick-row-text";
    const label = ownerDocument.createElement("span");
    label.className = "zeta-quick-pick-row-label";
    label.textContent = item.label;
    text.append(label);
    appendOptionalText(
      text,
      item.description,
      "zeta-quick-pick-row-description",
      ownerDocument,
    );
    appendOptionalText(
      text,
      item.detail,
      "zeta-quick-pick-row-detail",
      ownerDocument,
    );
    content.append(text);
    if (item.keybinding) {
      const keybinding = ownerDocument.createElement("kbd");
      keybinding.className = "zeta-quick-pick-row-keybinding";
      keybinding.textContent = item.keybinding;
      content.append(keybinding);
    }
    return content;
  }
}

export function filterQuickPickItems<TItem extends IQuickPickItem>(
  items: readonly TItem[],
  query: string,
): readonly TItem[] {
  const tokens = normalize(query).split(/\s+/).filter(Boolean);
  if (tokens.length === 0) return [...items];
  return items
    .map((item, index) => ({
      item,
      index,
      score: scoreItem(item, tokens),
    }))
    .filter((entry) => entry.score >= 0)
    .sort((left, right) =>
      right.score - left.score || left.index - right.index
    )
    .map((entry) => entry.item);
}

function scoreItem(
  item: IQuickPickItem,
  tokens: readonly string[],
): number {
  const label = normalize(item.label);
  const searchable = normalize(
    [item.label, item.description, item.detail].filter(Boolean).join(" "),
  );
  let score = 0;
  for (const token of tokens) {
    const labelScore = scoreSubsequence(label, token);
    const searchableScore = scoreSubsequence(searchable, token);
    const tokenScore = labelScore >= 0
      ? Math.max(labelScore + 40, searchableScore)
      : searchableScore;
    if (tokenScore < 0) return -1;
    score += tokenScore;
  }
  return score;
}

function scoreSubsequence(value: string, query: string): number {
  let valueIndex = 0;
  let score = 0;
  let previousMatch = -2;
  for (const character of query) {
    const match = value.indexOf(character, valueIndex);
    if (match < 0) return -1;
    score += match === previousMatch + 1 ? 8 : 2;
    if (match === 0 || /[\s._:/-]/.test(value[match - 1] ?? "")) {
      score += 6;
    }
    previousMatch = match;
    valueIndex = match + 1;
  }
  return score - Math.max(0, value.length - query.length) / 100;
}

function appendOptionalText(
  container: HTMLElement,
  value: string | undefined,
  className: string,
  ownerDocument: Document,
): void {
  if (!value) return;
  const element = ownerDocument.createElement("span");
  element.className = className;
  element.textContent = value;
  container.append(element);
}

function normalize(value: string): string {
  return value.trim().toLocaleLowerCase("en-US");
}
