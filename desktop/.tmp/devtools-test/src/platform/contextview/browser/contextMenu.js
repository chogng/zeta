import { Separator, } from "../../../base/common/actions.js";
import { createServiceIdentifier, } from "../../instantiation/common/instantiation.js";
export const IContextMenuService = createServiceIdentifier("contextMenuService");
export function resolveContextMenuActions(options, menuService) {
    if ("actions" in options)
        return trimSeparators(options.actions);
    const actions = menuService
        .getMenuActions(options.menuId, options.menuActionOptions)
        .flatMap(([, groupActions], index, groups) => [
        ...groupActions,
        ...(index < groups.length - 1 ? [new Separator()] : []),
    ]);
    return trimSeparators(actions);
}
function trimSeparators(actions) {
    const result = [];
    for (const action of actions) {
        if (action instanceof Separator &&
            (result.length === 0 || result[result.length - 1] instanceof Separator)) {
            continue;
        }
        result.push(action);
    }
    if (result[result.length - 1] instanceof Separator)
        result.pop();
    return result;
}
