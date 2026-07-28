import { toDisposable, } from "../../../base/common/lifecycle.js";
import { createServiceIdentifier, } from "../../instantiation/common/instantiation.js";
/** Stores realm-wide command definitions independently of their UI bindings. */
export class CommandRegistry {
    #commands = new Map();
    register(id, command) {
        if (this.#commands.has(id)) {
            throw new Error(`Command is already registered: ${id}`);
        }
        this.#commands.set(id, command);
        return toDisposable(() => {
            if (this.#commands.get(id) === command)
                this.#commands.delete(id);
        });
    }
    getCommand(id) {
        return this.#commands.get(id);
    }
}
/** Realm-wide command definitions populated by static contributions. */
export const CommandsRegistry = new CommandRegistry();
export const ICommandService = createServiceIdentifier("commandService");
