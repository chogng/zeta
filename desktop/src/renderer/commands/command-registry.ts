export type CommandId = "zeta.startTurn";
export class CommandRegistry {
  #commands = new Map<CommandId, () => Promise<void>>();
  register(id: CommandId, command: () => Promise<void>): void { this.#commands.set(id, command); }
  execute(id: CommandId): Promise<void> { const command = this.#commands.get(id); if (!command) return Promise.reject(new Error(`Unknown command: ${id}`)); return command(); }
}
