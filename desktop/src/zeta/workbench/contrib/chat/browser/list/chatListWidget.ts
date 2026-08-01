import { MarkdownElement } from "../../../../../base/browser/markdownRenderer.js";
import { ScrollableElement } from "../../../../../base/browser/ui/scrollbar/scrollableElement.js";
import { DisposableOwner, ResettableDisposableGroup } from "../../../../../base/common/lifecycle.js";
import type { IChatListItem } from "./chatListItems.js";

/** Renders the ordered user, Agent, reasoning, and tool items in one Chat pane. */
export class ChatListWidget extends DisposableOwner {
  readonly element: HTMLElement;
  private readonly scrollable: ScrollableElement;
  private readonly transcript: HTMLDivElement;
  private readonly renderedItems = this.own(new ResettableDisposableGroup());
  private visible = false;
  private shouldFollow = true;

  constructor(ownerDocument: Document) {
    super();
    this.scrollable = this.own(new ScrollableElement({
      ownerDocument,
      direction: "vertical",
      vertical: "auto",
      tabIndex: -1,
    }));
    this.element = this.scrollable.element;
    this.element.classList.add("zeta-chat-list-widget", "zeta-chat-transcript-scrollable");
    this.transcript = this.scrollable.contentElement;
    this.transcript.classList.add("zeta-chat-transcript");
    this.transcript.setAttribute("role", "log");
    this.transcript.setAttribute("aria-label", "Chat transcript");
    this.transcript.setAttribute("aria-live", "polite");
  }

  render(items: readonly IChatListItem[]): void {
    if (this.visible) this.captureFollowState();
    this.renderedItems.clear();
    this.transcript.replaceChildren(...items.map((item) => this.renderItem(item)));
    if (this.visible) this.layout();
  }

  setVisible(visible: boolean): void {
    if (this.visible === visible) return;
    if (!visible) this.captureFollowState();
    this.visible = visible;
    if (visible) this.layout();
  }

  private captureFollowState(): void {
    this.scrollable.layout();
    const state = this.scrollable.state;
    this.shouldFollow = state.scrollHeight - state.top - state.height < 48;
  }

  private layout(): void {
    this.scrollable.layout();
    if (this.shouldFollow) this.scrollable.scrollTo(0, this.scrollable.state.maximumTop);
  }

  private renderItem(item: IChatListItem): HTMLElement {
    const article = this.element.ownerDocument.createElement("article");
    article.className = `zeta-chat-item zeta-chat-item-${item.type}`;
    article.dataset.itemId = item.id;
    if (item.transient) article.dataset.transient = "true";
    if (item.isError) article.classList.add("error");
    const label = this.element.ownerDocument.createElement("div");
    label.className = "zeta-chat-item-label";
    label.textContent = itemLabel(item);
    article.append(label);
    if (item.type === "agentMessage" || item.type === "reasoning" || item.type === "plan") {
      const markdown = this.renderedItems.add(new MarkdownElement({
        ownerDocument: this.element.ownerDocument,
        markdown: item.text,
        breaks: true,
      }));
      article.append(markdown.element);
    } else {
      const content = this.element.ownerDocument.createElement("pre");
      content.textContent = item.text;
      article.append(content);
    }
    return article;
  }
}

function itemLabel(item: IChatListItem): string {
  switch (item.type) {
    case "userMessage":
    case "userImage":
      return "You";
    case "agentMessage":
      return "Zeta";
    case "reasoning":
      return "Reasoning";
    case "plan":
      return "Plan";
    case "toolCall":
      return "Tool call";
    case "toolResult":
      return item.isError ? "Tool error" : "Tool result";
  }
}
