/** Registry for editor commands that retrigger inline completions after execution. */
export abstract class TriggerInlineEditCommandsRegistry {
	private static REGISTERED_COMMANDS = new Set<string>();

	public static getRegisteredCommands(): readonly string[] {
		return [...TriggerInlineEditCommandsRegistry.REGISTERED_COMMANDS];
	}

	public static registerCommand(commandId: string): void {
		TriggerInlineEditCommandsRegistry.REGISTERED_COMMANDS.add(commandId);
	}
}
