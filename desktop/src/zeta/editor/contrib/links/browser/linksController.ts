import "./media/links.css";
import { addDisposableListener, stopEvent } from "../../../../base/browser/dom.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type LinkService, type LanguageLink } from "../common/links.js";
import { type TextPosition } from "../../../common/core/text.js";
import { type EditorViewport } from "../../../browser/view/editorViewport.js";

/** Resolves provider links on demand and delegates opening to the host callback. */
export class LinksController extends DisposableOwner {
  private request: AbortController | undefined;
  private links: readonly LanguageLink[] = [];
  private activeLink: LanguageLink | undefined;
  private hoverPosition: TextPosition | undefined;

  constructor(private readonly viewport: EditorViewport, private readonly service: LinkService, private readonly languageId: string, private readonly onOpenLink: (target: string) => void | Promise<void>, private readonly onError: (error: unknown) => void = error => console.error("Alpha link opening failed", error)) {
    super();
    this.own(addDisposableListener<PointerEvent>(viewport.element, "pointermove", event => this.update(event)));
    this.own(addDisposableListener(viewport.element, "pointerleave", () => this.clear()));
    this.own(addDisposableListener<PointerEvent>(viewport.element, "pointerdown", event => {
      if (event.button !== 0 || !this.activeLink) return;
      stopEvent(event);
      void this.open(this.activeLink.target);
    }));
    this.own(viewport.textModel.onDidChange(() => this.clear()));
  }

  private update(event: PointerEvent): void {
    const target = this.viewport.getNearestTargetAtClientPoint({ clientX: event.clientX, clientY: event.clientY });
    if (!target || target.kind !== "text") {
      this.clear();
      return;
    }
    this.hoverPosition = target.position;
    this.activeLink = this.links.find(link => link.range.containsPosition(target.position));
    this.viewport.element.classList.toggle("zeta-alpha-editor-link-target", this.activeLink !== undefined);
    if (this.links.length > 0) return;
    this.request?.abort();
    const request = this.request = new AbortController();
    void this.load(request);
  }

  private async load(request: AbortController): Promise<void> {
    try {
      const links = await this.service.provideLinks(this.languageId, request.signal);
      if (request.signal.aborted) return;
      this.links = links;
      this.activeLink = this.hoverPosition
        ? this.links.find(link => link.range.containsPosition(this.hoverPosition!))
        : undefined;
      this.viewport.element.classList.toggle("zeta-alpha-editor-link-target", this.activeLink !== undefined);
    } catch (error) {
      if (!request.signal.aborted) this.onError(error);
    }
  }

  private async open(target: string): Promise<void> {
    try {
      await this.onOpenLink(target);
    } catch (error) {
      this.onError(error);
    }
  }

  private clear(): void {
    this.request?.abort();
    this.request = undefined;
    this.links = [];
    this.activeLink = undefined;
    this.hoverPosition = undefined;
    this.viewport.element.classList.remove("zeta-alpha-editor-link-target");
  }
}
