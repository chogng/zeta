import { type IDisposable, DisposableOwner } from "../../../base/common/lifecycle.js";
import { type EditorViewportLayout } from "../../common/viewLayout/editorViewportModel.js";

/**
 * One browser-owned visual projection that is rendered by an EditorView host.
 *
 * A view part may own DOM or canvas resources, but it must consume the
 * layout supplied by the host rather than becoming a second viewport owner.
 */
export interface EditorViewPart extends IDisposable {
  render(layout: EditorViewportLayout): void;
}

/** Coordinates the ordered render pass for one editor's visual parts. */
export class EditorViewPartCollection extends DisposableOwner {
  private readonly parts: EditorViewPart[] = [];

  register<TPart extends EditorViewPart>(part: TPart): TPart {
    this.parts.push(part);
    this.own(part);
    return part;
  }

  render(layout: EditorViewportLayout): void {
    for (const part of this.parts) part.render(layout);
  }
}
