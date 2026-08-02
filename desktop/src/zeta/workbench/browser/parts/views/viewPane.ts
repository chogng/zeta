import "./views.css";
import { PaneView, type PaneViewOptions } from "../../../../base/browser/ui/splitview/paneView.js";
import type { IView } from "../../../common/views.js";

/** Runtime inputs supplied by a browser view container to every pane. */
export type IViewPaneOptions = PaneViewOptions;

/** Optional title content and actions projected together into a hosting Part. */
export interface PartTitleProjection {
  readonly content?: HTMLElement;
  readonly actions?: HTMLElement;
}

/** A titled, independently managed view hosted inside a workbench view container. */
export abstract class ViewPane extends PaneView implements IView {
  private visible = true;

  protected constructor(options: IViewPaneOptions) {
    super(options);
    this.element.classList.add("zeta-view-pane");
    this.element.dataset.viewId = options.id;
  }

  /** Optional title content and actions projected into the hosting Pane Composite Part. */
  get partTitleProjection(): PartTitleProjection | undefined {
    return undefined;
  }

  isVisible(): boolean {
    return this.visible;
  }

  setVisible(visible: boolean): void {
    if (this.visible === visible) return;
    this.visible = visible;
    this.element.hidden = !visible;
  }
}
