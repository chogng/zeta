import type { IContextMenuProvider } from "../../contextmenu.js";
import type { IAction } from "../../../common/actions.js";
import { Separator } from "../../../common/actions.js";
import { DisposableOwner } from "../../../common/lifecycle.js";
import { LxIcon } from "../../../common/lxicons.js";
import { ActionBar, type ActionBarOrientation, type ActionViewItemProvider } from "../actionbar/actionbar.js";
import { DropdownMenuActionViewItem } from "../dropdown/dropdownMenuActionViewItem.js";

export interface ToolBarOptions {
  readonly contextMenuProvider: IContextMenuProvider;
  readonly ownerDocument?: Document;
  readonly ariaLabel?: string;
  readonly orientation?: ActionBarOrientation;
  readonly actionViewItemProvider?: ActionViewItemProvider;
}

/**
 * Presents primary actions inline and secondary actions in a trailing menu.
 *
 * Callers own action classification. The toolbar owns the synthetic More
 * Actions item and delegates its secondary menu to the supplied provider.
 */
export class ToolBar extends DisposableOwner {
  readonly element: HTMLDivElement;
  readonly #actionBar: ActionBar;
  readonly #moreActions = new MoreActionsAction();
  #secondaryActions: readonly IAction[] = [];

  constructor(options: ToolBarOptions) {
    super();
    this.#actionBar = this.own(new ActionBar({
      ownerDocument: options.ownerDocument,
      ariaLabel: options.ariaLabel,
      orientation: options.orientation,
      actionViewItemProvider: (action) => {
        if (action === this.#moreActions) {
          return new MoreActionsViewItem(
            action,
            () => this.#secondaryActions,
            options.contextMenuProvider,
          );
        }
        return options.actionViewItemProvider?.(action);
      },
    }));
    this.element = this.#actionBar.element;
    this.element.classList.add("zeta-toolbar");
  }

  setActions(
    primaryActions: readonly IAction[],
    secondaryActions: readonly IAction[] = [],
  ): void {
    const primary = cleanSeparators(primaryActions);
    this.#secondaryActions = cleanSeparators(secondaryActions);
    this.#actionBar.setActions(
      this.#secondaryActions.length > 0
        ? [...primary, this.#moreActions]
        : primary,
    );
  }

}

class MoreActionsAction implements IAction {
  readonly id = "zeta.toolbar.moreActions";
  readonly label = "More Actions";
  readonly tooltip = "More Actions";
  readonly icon = LxIcon.ellipsis;
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
