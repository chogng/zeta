import {
  type IDisposable,
  toDisposable,
} from "../../../base/common/lifecycle.js";
import type { Event } from "../../../base/common/event.js";
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

/** A command invocation observed immediately around its handler call. */
export interface ICommandEvent {
  readonly commandId: CommandId;
  readonly args: readonly unknown[];
}

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
  readonly onWillExecuteCommand: Event<ICommandEvent>;
  readonly onDidExecuteCommand: Event<ICommandEvent>;
  executeCommand<T = unknown>(
    id: CommandId,
    ...args: readonly unknown[]
  ): Promise<T>;
}

export const ICommandService =
  createServiceIdentifier<ICommandService>("commandService");
