import { Emitter } from "../../../../base/common/event.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { CommandsRegistry, } from "../../../../platform/commands/common/commands.js";
/** Executes registered commands with the services of one Workbench window. */
export class CommandService extends DisposableOwner {
    #accessor;
    #registry;
    #onWillExecuteCommand = this.own(new Emitter());
    #onDidExecuteCommand = this.own(new Emitter());
    onWillExecuteCommand = this.#onWillExecuteCommand.event;
    onDidExecuteCommand = this.#onDidExecuteCommand.event;
    constructor(accessor, registry = CommandsRegistry) {
        super();
        this.#accessor = accessor;
        this.#registry = registry;
    }
    async executeCommand(id, ...args) {
        const command = this.#registry.getCommand(id);
        if (!command)
            throw new Error(`Unknown command: ${id}`);
        const event = { commandId: id, args };
        this.#onWillExecuteCommand.fire(event);
        const result = command(this.#accessor, ...args);
        this.#onDidExecuteCommand.fire(event);
        return await result;
    }
}
