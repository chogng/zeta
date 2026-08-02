import "./panelpart.css";
import type { IContextMenuProvider } from "../../../../base/browser/contextmenu.js";
import { ViewContainerLocation } from "../../../common/views.js";
import type { IViewDescriptorService } from "../../../services/views/common/viewDescriptorService.js";
import { PaneCompositePart } from "../paneCompositePart.js";

/** Construction inputs for the bottom Panel Composite host. */
export interface PanelPartOptions {
  readonly ownerDocument: Document;
  readonly viewDescriptorService: IViewDescriptorService;
  readonly contextMenuProvider?: IContextMenuProvider;
}

/** Bottom tool region with Panel tabs and a contextual title toolbar. */
export class PanelPart extends PaneCompositePart {
  override get minimumHeight(): number { return 80; }

  constructor(options: PanelPartOptions) {
    super({
      ownerDocument: options.ownerDocument,
      viewDescriptorService: options.viewDescriptorService,
      id: "panel",
      location: ViewContainerLocation.Panel,
      ariaLabel: "Panel",
      viewsAriaLabel: "Panel views",
      compositeBarPresentation: "label",
      compositeBarContextMenuProvider: options.contextMenuProvider,
    });
    this.titleElement.classList.add("zeta-panel-title-control");
    this.titleActionsSlotElement.classList.add("zeta-panel-title-actions");
    this.own(this.compositeBar.onDidChangeOverflowActions(() => {
      this.updateTitleActions();
    }));
  }

  override showComposite(compositeId: string): void {
    this.getComposite(this.activeCompositeId ?? "")
      ?.setTitleSecondaryActions([]);
    super.showComposite(compositeId);
    this.titleActionsSlotElement.replaceChildren(
      ...optionalElement(this.getComposite(compositeId)?.partTitleActionsElement),
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
