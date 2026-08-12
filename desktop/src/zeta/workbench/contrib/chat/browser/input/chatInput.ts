import type { AgentResponse, ModelCatalogEntry, SkillCommandDefinition, SlashCommandDefinition, TurnInteraction } from "../../../../services/chat/common/chatService.js";
import type { SkillReference } from "../../../../../platform/skills/common/skillApi.js";
import type { ModelRef } from "../../../../services/sessions/common/sessionService.js";

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
  readonly skillCommands: readonly SkillCommandDefinition[];
  readonly selectedModel?: ModelRef;
  readonly interaction?: TurnInteraction;
}

/** Operations that the input area may request from its owning Chat pane. */
export interface ChatInputDelegate {
  send(text: string, skills?: readonly SkillReference[]): Promise<void>;
  executeCommand(invocation: ChatInputCommandInvocation): Promise<void>;
  interrupt(): Promise<void>;
  selectModel(model: ModelRef): Promise<void>;
  resolveInteraction(response: AgentResponse): Promise<void>;
}
