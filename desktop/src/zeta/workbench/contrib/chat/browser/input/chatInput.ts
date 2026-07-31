import type { AgentResponse, ModelCatalogEntry, ModelRef, SlashCommandDefinition, TurnInteraction } from "../../../../../../../generated/app-server/types.js";

export type ChatInputPhase = "loading" | "ready" | "submitting" | "error";

export interface ChatInputCommandInvocation {
  readonly commandId: string;
  readonly argumentsText: string;
}

/** State required to render the input area for the selected Thread. */
export interface ChatInputState {
  readonly phase: ChatInputPhase;
  readonly error?: string;
  readonly canInterrupt: boolean;
  readonly models: readonly ModelCatalogEntry[];
  readonly slashCommands: readonly SlashCommandDefinition[];
  readonly selectedModel?: ModelRef;
  readonly interaction?: TurnInteraction;
}

/** Operations that the input area may request from its owning Chat pane. */
export interface ChatInputDelegate {
  send(text: string): Promise<void>;
  executeCommand(invocation: ChatInputCommandInvocation): Promise<void>;
  interrupt(): Promise<void>;
  selectModel(model: ModelRef): Promise<void>;
  resolveInteraction(response: AgentResponse): Promise<void>;
}
