import "./blockDecorations.css";
import { h } from "../../../../base/browser/dom.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type EditorViewportLayout } from "../../../common/viewLayout/editorViewportModel.js";
import { type ResolvedDecoration } from "../decorations/decorationPresentation.js";
import { type ViewportOverlayContext } from "../viewportOverlay/viewportOverlayPresentation.js";
import { type EditorViewPart } from "../viewPart.js";
import { resolveAsterBlockDecorationGeometry } from "./blockDecorationsProjection.js";

export interface BlockDecorationsPartOptions {
  readonly container: HTMLElement;
  readonly readOverlayContext: (layout: EditorViewportLayout) => ViewportOverlayContext;
  readonly readDecorations: (layout: EditorViewportLayout) => readonly ResolvedDecoration[];
}

/** Owns block-level decoration backgrounds and outlines in content coordinates. */
export class BlockDecorationsPart extends DisposableOwner implements EditorViewPart {
  readonly element: HTMLDivElement;
  private readonly readOverlayContext: (layout: EditorViewportLayout) => ViewportOverlayContext;
  private readonly readDecorations: (layout: EditorViewportLayout) => readonly ResolvedDecoration[];
  private readonly blocks: HTMLDivElement[] = [];

  constructor(options: BlockDecorationsPartOptions) {
    super();
    this.readOverlayContext = options.readOverlayContext;
    this.readDecorations = options.readDecorations;
    this.element = h(options.container.ownerDocument, "div");
    this.element.className = "aster-editor-block-decorations";
    this.element.setAttribute("role", "presentation");
    this.element.setAttribute("aria-hidden", "true");
    options.container.append(this.element);
    this.defer(() => this.element.remove());
  }

  render(layout: EditorViewportLayout): void {
    const context = this.readOverlayContext(layout);
    if (context.visualLineProjection.modelVersion !== context.model.version) return;
    this.element.style.width = `${layout.contentSize.width}px`;
    this.element.style.height = `${layout.contentSize.height}px`;
    let count = 0;
    for (const decoration of this.readDecorations(layout)) {
      if (!decoration.blockDecoration) continue;
      const geometry = resolveAsterBlockDecorationGeometry(context, layout, decoration);
      if (!geometry) continue;
      const block = this.blocks[count] ?? this.createBlock();
      block.className = "aster-editor-block-decoration";
      for (const className of decoration.blockDecoration.className.trim().split(/\s+/u)) {
        block.classList.add(className);
      }
      block.dataset.decorationId = String(decoration.id);
      block.style.left = `${geometry.left}px`;
      block.style.width = `${geometry.width}px`;
      block.style.top = `${geometry.top - geometry.padding[0]}px`;
      block.style.height = `${geometry.bottom - geometry.top + geometry.padding[0] + geometry.padding[2]}px`;
      count += 1;
    }
    for (let index = count; index < this.blocks.length; index += 1) this.blocks[index]!.remove();
    this.blocks.length = count;
  }

  private createBlock(): HTMLDivElement {
    const block = h(this.element.ownerDocument, "div");
    this.element.append(block);
    this.blocks.push(block);
    return block;
  }
}
