/** A non-interactive separator between groups of actions. */
export class Separator {
    static ID = "zeta.actions.separator";
    static join(...actionLists) {
        const result = [];
        for (const actions of actionLists) {
            if (actions.length === 0)
                continue;
            if (result.length > 0)
                result.push(new Separator());
            result.push(...actions);
        }
        return result;
    }
    id = Separator.ID;
    label = "";
    tooltip = "";
    enabled = false;
    checked = undefined;
    run() { }
}
/** An action whose children are rendered as a nested menu. */
export class SubmenuAction {
    id;
    label;
    actions;
    icon;
    enabled = true;
    checked = undefined;
    tooltip = "";
    constructor(id, label, actions, icon) {
        this.id = id;
        this.label = label;
        this.actions = actions;
        this.icon = icon;
    }
    run() { }
}
