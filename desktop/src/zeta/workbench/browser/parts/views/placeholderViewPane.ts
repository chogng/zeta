import {
  ViewPane,
  type IViewPaneOptions,
} from "./viewPane.js";

/** Informational pane used while a View Container awaits its full feature. */
export class PlaceholderViewPane extends ViewPane {
  constructor(
    message: string,
    options: IViewPaneOptions,
  ) {
    super(options);
    const placeholder = options.ownerDocument.createElement("p");
    placeholder.className = "zeta-view-pane-placeholder";
    placeholder.textContent = message;
    this.contentElement.append(placeholder);
  }
}
