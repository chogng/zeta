import {
  type IDisposable,
  toDisposable,
} from "../../../base/common/lifecycle.js";
import type {
  ServicesAccessor,
} from "../../instantiation/common/instantiation.js";
import {
  createServiceIdentifier,
} from "../../instantiation/common/instantiation.js";

export type CommandId = string;
export type CommandHandler = (
  accessor: ServicesAccessor,
  ...args: readonly unknown[]
) => unknown;

/** Stores realm-wide command definitions independently of their UI bindings. */
export class CommandRegistry {
  readonly #commands = new Map<CommandId, CommandHandler>();

  register(id: CommandId, command: CommandHandler): IDisposable {
    if (this.#commands.has(id)) {
      throw new Error(`Command is already registered: ${id}`);
    }
    this.#commands.set(id, command);
    return toDisposable(() => {
      if (this.#commands.get(id) === command) this.#commands.delete(id);
    });
  }

  getCommand(id: CommandId): CommandHandler | undefined {
    return this.#commands.get(id);
  }
}

/** Realm-wide command definitions populated by static contributions. */
export const CommandsRegistry = new CommandRegistry();

export interface ICommandService {
  executeCommand<T = unknown>(
    id: CommandId,
    ...args: readonly unknown[]
  ): Promise<T>;
}

export const ICommandService =
  createServiceIdentifier<ICommandService>("commandService");

/** Executes registered commands with the services of one workbench window. */
export class CommandService implements ICommandService {
  readonly #accessor: ServicesAccessor;
  readonly #registry: CommandRegistry;

  constructor(
    accessor: ServicesAccessor,
    registry: CommandRegistry = CommandsRegistry,
  ) {
    this.#accessor = accessor;
    this.#registry = registry;
  }

  async executeCommand<T = unknown>(
    id: CommandId,
    ...args: readonly unknown[]
  ): Promise<T> {
    const command = this.#registry.getCommand(id);
    if (!command) throw new Error(`Unknown command: ${id}`);
    return await command(this.#accessor, ...args) as T;
  }
}
