import { Action2, } from "../../../platform/actions/common/actions.js";
import { INativeHostService } from "../../common/services.js";
export const ToggleDeveloperToolsCommandId = "workbench.action.toggleDevTools";
/** Toggles the developer tools for the active Electron window. */
export class ToggleDeveloperToolsAction extends Action2 {
    constructor() {
        super({
            id: ToggleDeveloperToolsCommandId,
            title: "Developer: Toggle Developer Tools",
            f1: true,
        });
    }
    run(accessor) {
        return accessor.get(INativeHostService).toggleDeveloperTools();
    }
}
