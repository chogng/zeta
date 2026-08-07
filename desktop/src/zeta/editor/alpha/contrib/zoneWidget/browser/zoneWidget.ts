import "./media/zoneWidget.css";
import { DisposableOwner } from "../../../../../base/common/lifecycle.js";
import { type TextPosition } from "../../../common/core/text.js";
import { type EditorViewport } from "../../../browser/view/editorViewport.js";

export interface ZoneWidgetOptions { readonly anchor: TextPosition; readonly createContent: (document: Document) => HTMLElement; readonly className?: string; }

/** Anchors a transient interactive widget to a model position and follows viewport layout. */
export class ZoneWidget extends DisposableOwner {
  readonly element: HTMLDivElement;
  private content: HTMLElement;

  constructor(private readonly viewport: EditorViewport, options: ZoneWidgetOptions) {
    super();
    viewport.textModel.offsetAt(options.anchor);
    this.element = viewport.element.ownerDocument.createElement("div");
    this.element.className = `zeta-alpha-editor-zone-widget${options.className ? ` ${options.className}` : ""}`;
    this.content = options.createContent(viewport.element.ownerDocument);
    this.element.append(this.content);
    this.element.hidden = true;
    viewport.element.append(this.element);
    this.defer(() => this.element.remove());
    this.own(viewport.onDidChangeLayout(() => this.layout(options.anchor)));
    this.layout(options.anchor);
  }

  show(): void { this.element.hidden = false; this.layout(); }
  hide(): void { this.element.hidden = true; }
  setContent(content: HTMLElement): void {
    this.content.replaceWith(content);
    this.content = content;
  }

  private layout(anchor = this.currentAnchor): void {
    if (!anchor) return;
    const coordinates = this.viewport.getPositionContentCoordinates(anchor);
    const scroll = this.viewport.viewportLayout.scrollPosition;
    this.element.style.left = `${Math.max(4, coordinates.left - scroll.left)}px`;
    this.element.style.top = `${Math.max(4, coordinates.top - scroll.top + coordinates.height + 2)}px`;
    this.currentAnchor = anchor;
  }

  private currentAnchor: TextPosition | undefined;
}
