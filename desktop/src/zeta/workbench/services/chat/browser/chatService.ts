import type { AgentResponse as AgentResponseDto, InputItem, SkillRef as SkillRefDto, Thread as ThreadDto, ThreadUpdateEnvelope as ThreadUpdateEnvelopeDto } from "../../../../../../generated/app-server/types.js";
import { Emitter } from "../../../../base/common/event.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { createUuid } from "../../../../base/common/uuid.js";
import type { IAppServerApi, IServerEventApi } from "../../../../platform/app-server/common/appServerApi.js";
import type { IModelApi, IThreadApi, ITurnApi } from "../../../../platform/sessions/common/sessionApi.js";
import type { ISkillApi } from "../../../../platform/skills/common/skillApi.js";
import type { SessionId, ThreadId } from "../../../../sessions/services/sessions/common/session.js";
import type { IChatService, InterruptTurnOptions, ModelCatalogEntry, ResolveInteractionOptions, SkillCommandDefinition, SlashCommandDefinition, StartTurnOptions, Thread, ThreadSubscription, ThreadUpdateEnvelope } from "../common/chatService.js";

export interface ChatServiceOptions {
  readonly modelApi: IModelApi;
  readonly threadApi: IThreadApi;
  readonly turnApi: ITurnApi;
  readonly skillApi: ISkillApi;
  readonly appServerApi: IAppServerApi;
  readonly eventApi: IServerEventApi;
}

/** App Server-backed implementation of the frontend Chat service. */
export class ChatService extends DisposableOwner implements IChatService {
  private readonly _onDidUpdateThread = this.own(new Emitter<ThreadUpdateEnvelope>());
  private readonly _onDidBecomeReady = this.own(new Emitter<void>());
  private readonly _onDidChangeSkills = this.own(new Emitter<void>());

  readonly onDidUpdateThread = this._onDidUpdateThread.event;
  readonly onDidBecomeReady = this._onDidBecomeReady.event;
  readonly onDidChangeSkills = this._onDidChangeSkills.event;

  constructor(private readonly options: ChatServiceOptions) {
    super();
    const events = options.eventApi.subscribe((event) => {
      if (event.method === "session/thread/update") this._onDidUpdateThread.fire(toThreadUpdate(event.params));
      if (event.method === "skills/changed") this._onDidChangeSkills.fire();
    });
    this.defer(() => events.dispose());
    const connection = options.appServerApi.onConnectionState((state) => {
      if (state === "ready") this._onDidBecomeReady.fire();
    });
    this.defer(() => connection.dispose());
  }

  async listModels(): Promise<readonly ModelCatalogEntry[]> {
    const result = await this.options.modelApi.list();
    return result.models.map((entry) => ({ model: { ...entry.model }, displayName: entry.displayName }));
  }

  async listSlashCommands(): Promise<readonly SlashCommandDefinition[]> {
    const commands = await this.options.appServerApi.getSlashCommands();
    return commands.map((command) => ({ ...command }));
  }

  async listSkillCommands(): Promise<readonly SkillCommandDefinition[]> {
    const catalog = await this.options.skillApi.list("cached");
    const counts = new Map<string, number>();
    for (const skill of catalog.skills.filter(skill => skill.enabled && skill.compatible)) counts.set(skill.id.name, (counts.get(skill.id.name) ?? 0) + 1);
    return catalog.skills
      .filter(skill => skill.enabled && skill.compatible && counts.get(skill.id.name) === 1)
      .map(skill => ({
        name: skill.id.name,
        description: skill.description,
        source: skill.id.source,
        skill: { id: { ...skill.id }, version: { type: "pinnedDigest", digest: skill.contentDigest } },
      }));
  }

  async readThread(sessionId: SessionId, threadId: ThreadId): Promise<Thread> {
    return toThread((await this.options.threadApi.read({ sessionId, threadId })).thread);
  }

  async subscribeThread(sessionId: SessionId, threadId: ThreadId, afterSequence: number): Promise<ThreadSubscription> {
    const result = await this.options.threadApi.subscribe({ sessionId, threadId, afterSequence });
    return { thread: toThread(result.thread), updates: result.updates.map(toThreadUpdate) };
  }

  unsubscribeThread(sessionId: SessionId, threadId: ThreadId): Promise<void> {
    return this.options.threadApi.unsubscribe({ sessionId, threadId });
  }

  async startTurn(options: StartTurnOptions): Promise<void> {
    const input: InputItem[] = [
      ...(options.skills ?? []).map(skill => ({ type: "skill" as const, skill: skill as SkillRefDto })),
      { type: "text", text: options.text },
    ];
    await this.options.turnApi.start({ commandId: commandId("turn"), sessionId: options.sessionId, threadId: options.threadId, expectedSequence: options.expectedSequence, approvalMode: "askPermissions", input });
  }

  async interruptTurn(options: InterruptTurnOptions): Promise<void> {
    await this.options.turnApi.interrupt({ commandId: commandId("interrupt"), ...options });
  }

  async resolveInteraction(options: ResolveInteractionOptions): Promise<void> {
    await this.options.turnApi.resolveInteraction({ commandId: commandId("interaction"), ...options, response: toAgentResponse(options.response) });
  }
}

function toThread(thread: ThreadDto): Thread {
  return {
    sessionId: thread.sessionId,
    threadId: thread.threadId,
    title: thread.title,
    status: thread.status,
    sequence: thread.sequence,
    turns: thread.turns.map((turn) => ({ turnId: turn.turnId, status: turn.status, model: turn.model ? { ...turn.model } : turn.model, items: turn.items.map((item) => ({ ...item })) })),
  };
}

function toThreadUpdate(update: ThreadUpdateEnvelopeDto): ThreadUpdateEnvelope {
  return update as unknown as ThreadUpdateEnvelope;
}

function toAgentResponse(response: ResolveInteractionOptions["response"]): AgentResponseDto {
  switch (response.type) {
    case "approval": return { type: "approval", response: { ...response.response } };
    case "userInput": return { type: "userInput", response: { answers: { ...response.response.answers } } };
    case "dynamicTool": return { type: "dynamicTool", response: { ...response.response, content: response.response.content.map((output) => ({ ...output })) } };
  }
}

function commandId(kind: string): string { return `desktop-${kind}-${createUuid()}`; }
