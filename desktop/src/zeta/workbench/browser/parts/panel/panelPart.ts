import "./panelpart.css";
import type { IContextMenuProvider } from "../../../../base/browser/contextmenu.js";
import type { IDimension } from "../../../../base/browser/geometry.js";
import { type Event } from "../../../../base/common/event.js";
import { ViewContainerLocation } from "../../../common/views.js";
import type { IViewDescriptorService } from "../../../services/views/common/viewDescriptorService.js";
import { CompositePart } from "../compositePart.js";
import { CompositeBar, type CompositeBarSelectionEvent } from "../compositebar/compositeBar.js";

/** Construction inputs for the bottom Panel Composite host. */
export interface PanelPartOptions {
  readonly ownerDocument: Document;
  readonly viewDescriptorService: IViewDescriptorService;
  readonly contextMenuProvider?: IContextMenuProvider;
}

/** Bottom tool region with Panel tabs and a contextual title toolbar. */
export class PanelPart extends CompositePart {
  readonly compositeBar: CompositeBar;
  readonly onDidSelectComposite: Event<CompositeBarSelectionEvent>;
  private readonly actionsElement: HTMLDivElement;

  override get minimumHeight(): number { return 80; }

  constructor(options: PanelPartOptions) {
    super("panel", options.ownerDocument);
    this.element.setAttribute("aria-label", "Panel");
    this.compositeBar = this.own(new CompositeBar({
      ownerDocument: options.ownerDocument,
      viewDescriptorService: options.viewDescriptorService,
      location: ViewContainerLocation.Panel,
      ariaLabel: "Panel views",
      presentation: "label",
      contextMenuProvider: options.contextMenuProvider,
    }));
    this.onDidSelectComposite = this.compositeBar.onDidSelectComposite;
    const titleControl = options.ownerDocument.createElement("div");
    titleControl.className = "zeta-panel-title-control";
    this.actionsElement = options.ownerDocument.createElement("div");
    this.actionsElement.className = "zeta-panel-title-actions";
    titleControl.append(this.compositeBar.element, this.actionsElement);
    this.contentElement.before(titleControl);
    this.own(this.compositeBar.onDidChangeOverflowActions(() => {
      this.updateTitleActions();
    }));
  }

  setActiveComposite(compositeId: string): void {
    this.compositeBar.setActiveComposite(compositeId);
  }

  override layout(_dimension: IDimension): void {
    this.compositeBar.layout();
  }

  override showComposite(compositeId: string): void {
    this.getComposite(this.activeCompositeId ?? "")
      ?.setTitleSecondaryActions([]);
    super.showComposite(compositeId);
    this.actionsElement.replaceChildren(
      ...optionalElement(this.getComposite(compositeId)?.titleActionsElement),
    );
    this.updateTitleActions();
  }

  private updateTitleActions(): void {
    const composite = this.getComposite(this.activeCompositeId ?? "");
    const usesExternalOverflow = composite?.setTitleSecondaryActions(
      this.compositeBar.getOverflowActions(),
    ) === true;
    this.compositeBar.setOverflowPresentation(
      usesExternalOverflow ? "external" : "inline",
    );
  }
}

function optionalElement(element: HTMLElement | undefined): HTMLElement[] {
  return element ? [element] : [];
}
