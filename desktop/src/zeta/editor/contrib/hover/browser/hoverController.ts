import "./media/hover.css";
import { addDisposableListener } from "../../../../base/browser/dom.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type HoverService, type LanguageHover } from "../common/hover.js";
import { type TextPosition } from "../../../common/core/text.js";
import { type EditorViewport } from "../../../browser/view/editorViewport.js";

/** Projects provider-backed hover content into an editor-local, non-modal widget. */
export class HoverController extends DisposableOwner {
  private readonly element: HTMLDivElement;
  private request: AbortController | undefined;
  private timer: ReturnType<typeof setTimeout> | undefined;

  constructor(private readonly viewport: EditorViewport, private readonly service: HoverService, private readonly languageId: string) {
    super();
    this.element = viewport.element.ownerDocument.createElement("div");
    this.element.className = "zeta-alpha-editor-hover";
    this.element.hidden = true;
    this.element.setAttribute("role", "tooltip");
    viewport.element.append(this.element);
    this.defer(() => this.element.remove());
    this.own(addDisposableListener<PointerEvent>(viewport.element, "pointermove", event => this.schedule(event)));
    this.own(addDisposableListener(viewport.element, "pointerleave", () => this.hide()));
    this.own(addDisposableListener(viewport.element, "scroll", () => this.hide()));
    this.own(viewport.textModel.onDidChange(() => this.hide()));
  }

  private schedule(event: PointerEvent): void {
    const target = this.viewport.getNearestTargetAtClientPoint({ clientX: event.clientX, clientY: event.clientY });
    if (!target || target.kind !== "text") {
      this.hide();
      return;
    }
    this.cancelRequest();
    if (this.timer !== undefined) clearTimeout(this.timer);
    this.timer = setTimeout(() => {
      this.timer = undefined;
      void this.show(target.position);
    }, 300);
  }

  private async show(position: TextPosition): Promise<void> {
    const request = this.request = new AbortController();
    try {
      const hover = await this.service.provideHover(this.languageId, position, request.signal);
      if (request.signal.aborted || !hover) return;
      this.render(hover, position);
    } catch {
      if (!request.signal.aborted) this.hide();
    }
  }

  private render(hover: LanguageHover, position: TextPosition): void {
    this.element.replaceChildren(...hover.contents.map(content => {
      const node = this.element.ownerDocument.createElement("div");
      node.className = "zeta-alpha-editor-hover-content";
      node.textContent = typeof content === "string" ? content : content.value;
      return node;
    }));
    const coordinates = this.viewport.getPositionContentCoordinates(hover.range?.start ?? position);
    const bounds = this.viewport.element.getBoundingClientRect();
    const width = Math.min(480, Math.max(160, bounds.width - 16));
    this.element.style.maxWidth = `${width}px`;
    this.element.style.left = `${Math.max(8, coordinates.left - this.viewport.viewportLayout.scrollPosition.left)}px`;
    this.element.style.top = `${Math.max(8, coordinates.top - this.viewport.viewportLayout.scrollPosition.top + coordinates.height + 4)}px`;
    this.element.hidden = false;
  }

  private hide(): void {
    this.cancelRequest();
    if (this.timer !== undefined) {
      clearTimeout(this.timer);
      this.timer = undefined;
    }
    this.element.hidden = true;
    this.element.replaceChildren();
  }

  private cancelRequest(): void {
    this.request?.abort();
    this.request = undefined;
  }
}
