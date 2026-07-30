import type { IContextMenuProvider } from "../../contextmenu.js";
import type { IAction } from "../../../common/actions.js";
import { Separator } from "../../../common/actions.js";
import { DisposableOwner } from "../../../common/lifecycle.js";
import { lxiconsLibrary } from "../../../common/lxiconsLibrary.js";
import { ActionBar, type ActionBarOrientation, type ActionViewItemProvider } from "../actionbar/actionbar.js";
import { DropdownMenuActionViewItem } from "../dropdown/dropdownMenuActionViewItem.js";

export interface ToolBarOptions {
  readonly contextMenuProvider: IContextMenuProvider;
  readonly ownerDocument?: Document;
  readonly ariaLabel?: string;
  readonly orientation?: ActionBarOrientation;
  readonly actionViewItemProvider?: ActionViewItemProvider;
  readonly presentation?: ToolBarPresentation;
  readonly highlightToggledItems?: boolean;
}

/** Component-owned visual adaptation selected by a toolbar host. */
export type ToolBarPresentation = "default" | "inherit-foreground";

/**
 * Presents primary actions inline and secondary actions in a trailing menu.
 *
 * Callers own action classification. The toolbar owns the synthetic More
 * Actions item and delegates its secondary menu to the supplied provider.
 */
export class ToolBar extends DisposableOwner {
  readonly element: HTMLDivElement;
  private readonly actionBar: ActionBar;
  private readonly moreActions = new MoreActionsAction();
  private secondaryActions: readonly IAction[] = [];

  constructor(options: ToolBarOptions) {
    super();
    this.actionBar = this.own(new ActionBar({
      ownerDocument: options.ownerDocument,
      ariaLabel: options.ariaLabel,
      orientation: options.orientation,
      highlightToggledItems: options.highlightToggledItems,
      actionViewItemProvider: (action) => {
        if (action === this.moreActions) {
          return new MoreActionsViewItem(
            action,
            () => this.secondaryActions,
            options.contextMenuProvider,
          );
        }
        return options.actionViewItemProvider?.(action);
      },
    }));
    this.element = this.actionBar.element;
    this.element.classList.add("zeta-toolbar", `zeta-toolbar-${options.presentation ?? "default"}`);
  }

  setActions(
    primaryActions: readonly IAction[],
    secondaryActions: readonly IAction[] = [],
  ): void {
    const primary = cleanSeparators(primaryActions);
    this.secondaryActions = cleanSeparators(secondaryActions);
    this.actionBar.setActions(
      this.secondaryActions.length > 0
        ? [...primary, this.moreActions]
        : primary,
    );
  }

}

class MoreActionsAction implements IAction {
  readonly id = "zeta.toolbar.moreActions";
  readonly label = "More Actions";
  readonly tooltip = "More Actions";
  readonly icon = lxiconsLibrary.ellipsis;
  readonly enabled = true;
  readonly checked = undefined;

  run(): void {}
}

class MoreActionsViewItem extends DropdownMenuActionViewItem {
  override render(container: HTMLElement): void {
    super.render(container);
    container.classList.add("zeta-toolbar-more-actions");
  }
}

function cleanSeparators(actions: readonly IAction[]): IAction[] {
  const result: IAction[] = [];
  for (const action of actions) {
    if (
      action instanceof Separator &&
      (result.length === 0 || result[result.length - 1] instanceof Separator)
    ) {
      continue;
    }
    result.push(action);
  }
  if (result[result.length - 1] instanceof Separator) result.pop();
  return result;
}
