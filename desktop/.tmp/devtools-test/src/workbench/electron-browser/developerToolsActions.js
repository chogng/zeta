import { Action2, registerAction2, } from "../../platform/actions/common/actions.js";
import { INativeHostService } from "../common/services.js";
export const ToggleDeveloperToolsCommandId = "workbench.action.toggleDevTools";
registerAction2(class ToggleDeveloperToolsAction extends Action2 {
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
});
