import { getKeybindingLabel } from "../../../../base/common/keybindingLabels.js";
import { Keybinding, logicalKey, } from "../../../../base/common/keybindings.js";
import { LxIcon } from "../../../../base/common/lxicons.js";
import { DisposableStore } from "../../../../base/common/lifecycle.js";
import { Action2, MenuId, MenuItemAction, registerAction2, } from "../../../../platform/actions/common/actions.js";
import { IMenuService, } from "../../../../platform/actions/common/menuService.js";
import { ICommandService, } from "../../../../platform/commands/common/commands.js";
import { IKeybindingService, } from "../../../../platform/keybinding/common/keybinding.js";
import { IQuickInputService, } from "../../../../platform/quickinput/common/quickInput.js";
export const ShowAllCommandsCommandId = "workbench.action.showCommands";
registerAction2(class ShowAllCommandsAction extends Action2 {
    constructor() {
        super({
            id: ShowAllCommandsCommandId,
            title: "Show All Commands",
            tooltip: "Manage",
            icon: LxIcon.gear,
            menu: {
                id: MenuId.TitleBar,
                group: "navigation",
                order: 20,
            },
            keybinding: {
                primary: Keybinding.single(logicalKey("p", {
                    primaryKey: true,
                    shiftKey: true,
                })),
                secondary: [Keybinding.single(logicalKey("F1"))],
            },
        });
    }
    run(accessor) {
        const commandService = accessor.get(ICommandService);
        const keybindingService = accessor.get(IKeybindingService);
        const menu = accessor.get(IMenuService).createMenu(MenuId.CommandPalette);
        const quickPick = accessor.get(IQuickInputService)
            .createQuickPick();
        const disposables = new DisposableStore();
        disposables.add(menu);
        disposables.add(quickPick);
        quickPick.placeholder = "Type the name of a command to run";
        const updateItems = () => {
            quickPick.items = menu.getActions()
                .flatMap(([, actions]) => actions)
                .filter((action) => action instanceof MenuItemAction && action.enabled)
                .map((action) => {
                const keybinding = keybindingService.lookupKeybinding(action.id);
                return {
                    commandId: action.id,
                    label: action.label,
                    description: action.id,
                    keybinding: keybinding
                        ? getKeybindingLabel(keybinding)
                        : undefined,
                };
            });
        };
        disposables.add(menu.onDidChange(updateItems));
        disposables.add(keybindingService.onDidUpdateKeybindings(updateItems));
        disposables.add(quickPick.onDidAccept((item) => {
            quickPick.hide();
            void commandService.executeCommand(item.commandId).catch((error) => {
                console.error(`Command Palette command failed: ${item.commandId}`, error);
            });
        }));
        disposables.add(quickPick.onDidHide(() => disposables.dispose()));
        updateItems();
        quickPick.show();
    }
});
