import type { AgentResponse, ModelCatalogEntry, ModelRef, TurnInteraction } from "../../../../../../../generated/app-server/types.js";

export type ChatInputPhase = "loading" | "ready" | "submitting" | "error";

/** State required to render the input area for the selected Thread. */
export interface ChatInputState {
  readonly phase: ChatInputPhase;
  readonly error?: string;
  readonly canInterrupt: boolean;
  readonly models: readonly ModelCatalogEntry[];
  readonly selectedModel?: ModelRef;
  readonly interaction?: TurnInteraction;
}

/** Operations that the input area may request from its owning Chat pane. */
export interface ChatInputDelegate {
  send(text: string): Promise<void>;
  interrupt(): Promise<void>;
  selectModel(model: ModelRef): Promise<void>;
  resolveInteraction(response: AgentResponse): Promise<void>;
}
