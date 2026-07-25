export type CommandId = string;
export type Command = () => Promise<void>;

/** Registers named UI actions independently of their menu, keyboard, or button bindings. */
export class CommandRegistry {
  #commands = new Map<CommandId, Command>();

  register(id: CommandId, command: Command): void {
    this.#commands.set(id, command);
  }

  execute(id: CommandId): Promise<void> {
    const command = this.#commands.get(id);
    if (!command) return Promise.reject(new Error(`Unknown command: ${id}`));
    return command();
  }
}
