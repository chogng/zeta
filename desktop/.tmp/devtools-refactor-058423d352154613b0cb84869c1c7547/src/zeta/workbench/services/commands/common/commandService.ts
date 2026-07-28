import { Emitter, type Event } from "../../../../base/common/event.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import {
  type CommandId,
  type CommandRegistry,
  CommandsRegistry,
  type ICommandEvent,
  type ICommandService,
} from "../../../../platform/commands/common/commands.js";
import type {
  ServicesAccessor,
} from "../../../../platform/instantiation/common/instantiation.js";

/** Executes registered commands with the services of one Workbench window. */
export class CommandService
  extends DisposableOwner
  implements ICommandService {
  readonly #accessor: ServicesAccessor;
  readonly #registry: CommandRegistry;
  readonly #onWillExecuteCommand = this.own(new Emitter<ICommandEvent>());
  readonly #onDidExecuteCommand = this.own(new Emitter<ICommandEvent>());

  readonly onWillExecuteCommand: Event<ICommandEvent> =
    this.#onWillExecuteCommand.event;
  readonly onDidExecuteCommand: Event<ICommandEvent> =
    this.#onDidExecuteCommand.event;

  constructor(
    accessor: ServicesAccessor,
    registry: CommandRegistry = CommandsRegistry,
  ) {
    super();
    this.#accessor = accessor;
    this.#registry = registry;
  }

  async executeCommand<T = unknown>(
    id: CommandId,
    ...args: readonly unknown[]
  ): Promise<T> {
    const command = this.#registry.getCommand(id);
    if (!command) throw new Error(`Unknown command: ${id}`);
    const event = { commandId: id, args };
    this.#onWillExecuteCommand.fire(event);
    const result = command(this.#accessor, ...args) as T | PromiseLike<T>;
    this.#onDidExecuteCommand.fire(event);
    return await result;
  }
}
