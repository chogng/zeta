/** Registry for editor commands that retrigger inline completions after execution. */
export abstract class TriggerInlineEditCommandsRegistry {
	private static readonly registeredCommands = new Set<string>();

	public static getRegisteredCommands(): readonly string[] {
		return [...TriggerInlineEditCommandsRegistry.registeredCommands];
	}

	public static registerCommand(commandId: string): void {
		if (typeof commandId !== 'string' || commandId.trim().length === 0) {
			throw new TypeError('Inline edit trigger command ID must be a non-empty string');
		}
		TriggerInlineEditCommandsRegistry.registeredCommands.add(commandId);
	}
}
