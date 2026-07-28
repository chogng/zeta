import type { Icon } from "./icon.js";

/** A resolved action that can be presented by menus, toolbars, and buttons. */
export interface IAction {
  readonly id: string;
  readonly label: string;
  readonly tooltip: string;
  readonly icon?: Icon;
  readonly enabled: boolean;
  readonly checked?: boolean;

  run(...args: readonly unknown[]): unknown;
}

/** A non-interactive separator between groups of actions. */
export class Separator implements IAction {
  static readonly ID = "zeta.actions.separator";

  static join(...actionLists: readonly IAction[][]): IAction[] {
    const result: IAction[] = [];
    for (const actions of actionLists) {
      if (actions.length === 0) continue;
      if (result.length > 0) result.push(new Separator());
      result.push(...actions);
    }
    return result;
  }

  readonly id = Separator.ID;
  readonly label = "";
  readonly tooltip = "";
  readonly enabled = false;
  readonly checked = undefined;

  run(): void {}
}

/** An action whose children are rendered as a nested menu. */
export class SubmenuAction implements IAction {
  readonly enabled = true;
  readonly checked = undefined;
  readonly tooltip = "";

  constructor(
    readonly id: string,
    readonly label: string,
    readonly actions: readonly IAction[],
    readonly icon?: Icon,
  ) {}

  run(): void {}
}
