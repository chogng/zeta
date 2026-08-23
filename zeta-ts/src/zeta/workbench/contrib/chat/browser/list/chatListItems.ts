import type { ThreadItem, Turn, TurnError } from "../../../../services/chat/common/chatService.js";

export type ChatTurnErrorAction =
	| { readonly type: "retry"; readonly label: string; readonly turnId: string }
	| { readonly type: "chooseModel"; readonly label: string }
	| { readonly type: "startNewChat"; readonly label: string }
	| { readonly type: "revise"; readonly label: string };

interface ChatTurnErrorListItemOptions {
	readonly actionsEnabled?: boolean;
}

/** Render-ready projection of one committed or transient Thread item. */
export interface IChatListItem {
	readonly id: string;
	readonly type: ThreadItem["type"] | "turnError";
	readonly text: string;
	readonly transient: boolean;
	readonly isError?: boolean;
	readonly label?: string;
	readonly detail?: string;
	readonly errorCode?: TurnError["code"];
	readonly action?: ChatTurnErrorAction;
}

/** Projects one durable Turn failure as a conversation item. */
export function chatTurnErrorListItem(turn: Turn, options: ChatTurnErrorListItemOptions = {}): IChatListItem | undefined {
	if (turn.status !== "failed") return undefined;
	const error = turn.error ?? undefined;
	const presentation = error ? turnErrorPresentation(turn.turnId, error) : undefined;
	return {
		id: `turn-error:${turn.turnId}`,
		type: "turnError",
		text: error?.message ?? "Turn failed",
		transient: false,
		isError: true,
		label: presentation?.label,
		detail: presentation?.detail,
		errorCode: error?.code,
		action: options.actionsEnabled === false ? undefined : presentation?.action,
	};
}

function turnErrorPresentation(turnId: string, error: TurnError): { readonly label: string; readonly detail: string; readonly action: ChatTurnErrorAction } {
	switch (error.code) {
		case "modelInvocationFailed":
			return retryPresentation(turnId, "Model error", "The model request may have failed temporarily.");
		case "contextOverflow":
			return {
				label: "Context limit",
				detail: "Automatic context recovery was exhausted. Start a new chat or send a smaller request.",
				action: { type: "startNewChat", label: "Start new chat" },
			};
		case "providerAuth":
			return {
				label: "Authentication",
				detail: "Choose a model with working credentials before sending another message.",
				action: { type: "chooseModel", label: "Choose another model" },
			};
		case "invalidRequest":
			return revisePresentation("Invalid request", "Revise the request or choose a different model.");
		case "invalidResponse":
			return retryPresentation(turnId, "Invalid response", "The model returned a response Zeta could not use.");
		case "completionPersistenceFailed":
			return retryPresentation(turnId, "Save failed", "The Turn completion could not be saved. Review the conversation before retrying.");
		case "interactionDeadlineElapsed":
			return retryPresentation(turnId, "Interaction expired", "The requested interaction expired before it received a response.");
		case "toolRepetition":
			return revisePresentation("Repeated tool failure", "The same tool and arguments failed five times. Ask Zeta to use a different approach or explain the blocker.");
		case "turnBudgetExhausted":
			return {
				label: "Turn budget",
				detail: "This Turn reached its configured resource budget. Start a new chat before continuing.",
				action: { type: "startNewChat", label: "Start new chat" },
			};
	}
}

function retryPresentation(turnId: string, label: string, detail: string): { readonly label: string; readonly detail: string; readonly action: ChatTurnErrorAction } {
	return { label, detail, action: { type: "retry", label: "Try again", turnId } };
}

function revisePresentation(label: string, detail: string): { readonly label: string; readonly detail: string; readonly action: ChatTurnErrorAction } {
	return { label, detail, action: { type: "revise", label: "Change approach" } };
}

/** Maps a Chat Thread item without interpreting untrusted content. */
export function chatListItem(item: ThreadItem, transient = false): IChatListItem {
	switch (item.type) {
		case "userMessage":
		case "agentMessage":
		case "reasoning":
		case "plan":
			return {
				id: item.itemId,
				type: item.type,
				text: item.text,
				transient,
			};
		case "userImage":
		case "userImageAttachment":
			return {
				id: item.itemId,
				type: item.type,
				text: "Image",
				transient,
			};
		case "toolCall":
			return {
				id: item.itemId,
				type: item.type,
				text: `${item.name}\n${item.argumentsJson}`,
				transient,
			};
		case "toolResult":
			return {
				id: item.itemId,
				type: item.type,
				text: item.text,
				transient,
				isError: item.isError,
			};
	}
}
