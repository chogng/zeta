import { Emitter, type Event } from "../../../../base/common/event.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import type { ITerminalProcessCommandStatusEvent, ITerminalProcessOutputChunk, ITerminalProcessService, TerminalProcessConnectionState } from "../../../../platform/terminal/common/terminalProcessService.js";
import type { ITerminalCommandStatusEvent, ITerminalCreateOptions, ITerminalDimensions, ITerminalInstance, ITerminalProfile, ITerminalService, TerminalInstanceState } from "../common/terminal.js";

const POLL_DELAY_MILLIS = 35;
const INPUT_BATCH_DELAY_MILLIS = 8;
const INPUT_BATCH_CHARACTERS = 16_384;
const MAX_INPUT_BATCH_BYTES = 60 * 1024;
const MAX_READ_CHUNKS = 128;

/** Browser Workbench owner of terminal instances and their process lifecycle. */
export class TerminalService extends DisposableOwner implements ITerminalService {
  private readonly processService: ITerminalProcessService;
  private readonly _instances: TerminalInstance[] = [];
  private readonly _onDidCreateInstance = this.own(new Emitter<ITerminalInstance>());
  private readonly _onDidDisposeInstance = this.own(new Emitter<ITerminalInstance>());
  private readonly _onDidChangeInstances = this.own(new Emitter<void>());
  private readonly _onDidChangeActiveInstance = this.own(new Emitter<ITerminalInstance | undefined>());
  private _activeInstance: TerminalInstance | undefined;
  private nextInstanceId = 1;
  private connectionState: TerminalProcessConnectionState = "ready";
  private connectionRevision = 0;

  readonly onDidCreateInstance: Event<ITerminalInstance> = this._onDidCreateInstance.event;
  readonly onDidDisposeInstance: Event<ITerminalInstance> = this._onDidDisposeInstance.event;
  readonly onDidChangeInstances: Event<void> = this._onDidChangeInstances.event;
  readonly onDidChangeActiveInstance: Event<ITerminalInstance | undefined> = this._onDidChangeActiveInstance.event;

  constructor(processService: ITerminalProcessService) {
    super();
    this.processService = processService;
    this.own(processService.onConnectionState((state) => {
      this.connectionRevision += 1;
      this.setConnectionState(state);
    }));
    const connectionRevision = this.connectionRevision;
    void processService.getConnectionState()
      .then((state) => {
        if (this.connectionRevision === connectionRevision) this.setConnectionState(state);
      })
      .catch(() => {
        if (this.connectionRevision === connectionRevision) this.setConnectionState("crashed");
      });
    this.defer(() => {
      for (const instance of [...this._instances]) {
        void instance.close().catch(() => {});
      }
      this._instances.length = 0;
      this._activeInstance = undefined;
    });
  }

  get instances(): readonly ITerminalInstance[] {
    return this._instances;
  }

  get activeInstance(): ITerminalInstance | undefined {
    return this._activeInstance;
  }

  async getProfiles(): Promise<readonly ITerminalProfile[]> {
    return this.processService.listProfiles();
  }

  async createTerminal(options: ITerminalCreateOptions): Promise<ITerminalInstance> {
    const created = await this.processService.create({
      rows: options.dimensions.rows,
      cols: options.dimensions.cols,
      profile: options.profile,
    });
    const instanceNumber = this.nextInstanceId++;
    const instance = this.own(new TerminalInstance(
      `terminal-instance-${instanceNumber}`,
      created.terminalId,
      terminalProfileTitle(created.profile),
      created.profile,
      this.processService,
      () => this.removeInstance(instance),
    ));
    this._instances.push(instance);
    this.refreshInstanceTitles();
    this._onDidCreateInstance.fire(instance);
    this.setActiveInstance(instance);
    instance.start();
    return instance;
  }

  async relaunchTerminal(instance: ITerminalInstance, dimensions: ITerminalDimensions): Promise<void> {
    if (!this._instances.includes(instance as TerminalInstance)) {
      throw new Error("Terminal must belong to this TerminalService");
    }
    await (instance as TerminalInstance).relaunch(dimensions);
  }

  setActiveInstance(instance: ITerminalInstance | undefined): void {
    if (instance !== undefined && !this._instances.includes(instance as TerminalInstance)) {
      throw new Error("Active terminal must belong to this TerminalService");
    }
    if (this._activeInstance === instance) return;
    this._activeInstance = instance as TerminalInstance | undefined;
    this._onDidChangeActiveInstance.fire(instance);
  }

  moveTerminal(instance: ITerminalInstance, targetIndex: number): void {
    const currentIndex = this._instances.indexOf(instance as TerminalInstance);
    if (currentIndex < 0) {
      throw new Error("Terminal must belong to this TerminalService");
    }
    this._instances.splice(currentIndex, 1);
    const insertionIndex = Math.min(Math.max(0, targetIndex), this._instances.length);
    this._instances.splice(insertionIndex, 0, instance as TerminalInstance);
    this.refreshInstanceTitles();
    this._onDidChangeInstances.fire();
  }

  async closeTerminal(instance: ITerminalInstance): Promise<void> {
    if (!this._instances.includes(instance as TerminalInstance)) return;
    await instance.close();
  }

  private removeInstance(instance: TerminalInstance): void {
    const index = this._instances.indexOf(instance);
    if (index < 0) return;
    const activeChanged = this._activeInstance === instance;
    this._instances.splice(index, 1);
    if (activeChanged) {
      this._activeInstance = this._instances.at(-1);
    }
    this.refreshInstanceTitles();
    this._onDidDisposeInstance.fire(instance);
    if (activeChanged) {
      this._onDidChangeActiveInstance.fire(this._activeInstance);
    }
  }

  private refreshInstanceTitles(): void {
    const instancesByProfile = new Map<string, TerminalInstance[]>();
    for (const instance of this._instances) {
      const profileInstances = instancesByProfile.get(instance.profile.profileId) ?? [];
      profileInstances.push(instance);
      instancesByProfile.set(instance.profile.profileId, profileInstances);
    }
    for (const profileInstances of instancesByProfile.values()) {
      const baseTitle = terminalProfileTitle(profileInstances[0].profile);
      for (const [index, instance] of profileInstances.entries()) {
        instance.setTitle(profileInstances.length === 1 ? baseTitle : `${baseTitle} ${index + 1}`);
      }
    }
  }

  private setConnectionState(state: TerminalProcessConnectionState): void {
    if (this.connectionState === state) return;
    this.connectionState = state;
    if (state === "ready") return;
    for (const instance of this._instances) {
      instance.disconnect();
    }
  }
}

class TerminalInstance extends DisposableOwner implements ITerminalInstance {
  private readonly processService: ITerminalProcessService;
  private readonly onClosed: () => void;
  private readonly _onDidWriteData = this.own(new Emitter<Uint8Array>());
  private readonly _onDidChangeCommandStatus = this.own(new Emitter<ITerminalCommandStatusEvent>());
  private readonly _onDidExit = this.own(new Emitter<number | undefined>());
  private readonly _onDidChangeState = this.own(new Emitter<TerminalInstanceState>());
  private _state: TerminalInstanceState = "running";
  private _exitCode: number | undefined;
  private nextSequence = 0;
  private nextCommandSequence = 0;
  private closed = false;
  private pendingInput = "";
  private inputTimer: ReturnType<typeof setTimeout> | undefined;
  private writeChain = Promise.resolve();
  private pendingDimensions: ITerminalDimensions | undefined;
  private resizeScheduled = false;
  private serverTerminalId: string;
  private _profile: ITerminalProfile;
  private _title: string;
  private pollGeneration = 0;

  readonly onDidWriteData: Event<Uint8Array> = this._onDidWriteData.event;
  readonly onDidChangeCommandStatus: Event<ITerminalCommandStatusEvent> = this._onDidChangeCommandStatus.event;
  readonly onDidExit: Event<number | undefined> = this._onDidExit.event;
  readonly onDidChangeState: Event<TerminalInstanceState> = this._onDidChangeState.event;

  constructor(
    readonly id: string,
    serverTerminalId: string,
    title: string,
    profile: ITerminalProfile,
    processService: ITerminalProcessService,
    onClosed: () => void,
  ) {
    super();
    this.serverTerminalId = serverTerminalId;
    this._profile = profile;
    this._title = title;
    this.processService = processService;
    this.onClosed = onClosed;
    this.defer(() => {
      if (this.inputTimer !== undefined) clearTimeout(this.inputTimer);
    });
  }

  get state(): TerminalInstanceState {
    return this._state;
  }

  get exitCode(): number | undefined {
    return this._exitCode;
  }

  get profile(): ITerminalProfile {
    return this._profile;
  }

  get title(): string {
    return this._title;
  }

  setTitle(title: string): void {
    this._title = title;
  }

  start(): void {
    const generation = ++this.pollGeneration;
    void this.poll(generation);
  }

  write(data: string): void {
    if (this.closed || this._state !== "running" || data.length === 0) return;
    this.pendingInput += data;
    if (this.pendingInput.length >= INPUT_BATCH_CHARACTERS) {
      this.flushInput();
      return;
    }
    if (this.inputTimer !== undefined) return;
    this.inputTimer = setTimeout(() => {
      this.inputTimer = undefined;
      this.flushInput();
    }, INPUT_BATCH_DELAY_MILLIS);
  }

  resize(dimensions: ITerminalDimensions): void {
    if (this.closed || this._state !== "running") return;
    this.pendingDimensions = dimensions;
    if (this.resizeScheduled) return;
    this.resizeScheduled = true;
    queueMicrotask(() => {
      this.resizeScheduled = false;
      const pending = this.pendingDimensions;
      this.pendingDimensions = undefined;
      if (!pending || this.closed || this._state !== "running") return;
      const generation = this.pollGeneration;
      void this.processService.resize({
        terminalId: this.serverTerminalId,
        rows: pending.rows,
        cols: pending.cols,
      }).catch(() => {
        if (generation === this.pollGeneration && this._state === "running") {
          this.setState("error");
        }
      });
    });
  }

  async close(): Promise<void> {
    if (this.closed) return;
    this.closed = true;
    if (this.inputTimer !== undefined) {
      clearTimeout(this.inputTimer);
      this.inputTimer = undefined;
    }
    this.pendingInput = "";
    this.pollGeneration += 1;
    try {
      await this.processService.close({ terminalId: this.serverTerminalId });
    } finally {
      this.onClosed();
      this.dispose();
    }
  }

  disconnect(): void {
    if (this.closed || this._state === "exited" || this._state === "disconnected") return;
    this.pollGeneration += 1;
    this.clearPendingInput();
    this.setState("disconnected");
    this._onDidWriteData.fire(new TextEncoder().encode("\r\n[terminal connection lost; process was not preserved]\r\n"));
  }

  async relaunch(dimensions: ITerminalDimensions): Promise<void> {
    if (this.closed || this._state === "running") return;
    await this.processService.close({ terminalId: this.serverTerminalId }).catch(() => {});
    try {
      const created = await this.processService.create({
        rows: dimensions.rows,
        cols: dimensions.cols,
        profile: {
          type: "profile",
          profileId: this._profile.profileId,
        },
      });
      this.serverTerminalId = created.terminalId;
      this._profile = created.profile;
      this.nextSequence = 0;
      this.nextCommandSequence = 0;
      this._exitCode = undefined;
      this._onDidWriteData.fire(new TextEncoder().encode("\r\n[terminal relaunched]\r\n"));
      this.setState("running");
      this.start();
    } catch (error) {
      this.setState("error");
      throw error;
    }
  }

  private flushInput(): void {
    if (this.inputTimer !== undefined) {
      clearTimeout(this.inputTimer);
      this.inputTimer = undefined;
    }
    if (this.closed || this._state !== "running" || this.pendingInput.length === 0) return;
    const data = takeUtf8Prefix(this.pendingInput, MAX_INPUT_BATCH_BYTES);
    this.pendingInput = this.pendingInput.slice(data.length);
    const generation = this.pollGeneration;
    this.writeChain = this.writeChain
      .then(() => {
        if (generation !== this.pollGeneration || this._state !== "running") return;
        return this.processService.write({ terminalId: this.serverTerminalId, data });
      })
      .catch(() => {
        if (generation === this.pollGeneration && this._state === "running") {
          this.setState("error");
        }
      })
      .then(() => {
        if (generation === this.pollGeneration && this._state === "running" && this.pendingInput.length > 0) {
          this.flushInput();
        }
      });
  }

  private async poll(generation: number): Promise<void> {
    while (!this.closed && this._state === "running" && generation === this.pollGeneration) {
      try {
        const result = await this.processService.read({
          terminalId: this.serverTerminalId,
          afterSequence: this.nextSequence,
          afterCommandSequence: this.nextCommandSequence,
          maxChunks: MAX_READ_CHUNKS,
        });
        if (this.closed || this._state !== "running" || generation !== this.pollGeneration) return;
        if (result.outputGap) {
          this._onDidWriteData.fire(new TextEncoder().encode("\r\n[terminal output truncated]\r\n"));
        }
        this.emitReadResult(result.chunks, result.commandEvents);
        this.nextSequence = result.nextSequence;
        this.nextCommandSequence = result.nextCommandSequence;
        if (result.exited) {
          this._exitCode = result.exitCode ?? undefined;
          this.setState("exited");
          this._onDidExit.fire(this._exitCode);
          return;
        }
        if (result.chunks.length === 0) await delay(POLL_DELAY_MILLIS);
      } catch {
        if (!this.closed && generation === this.pollGeneration) this.setState("error");
        return;
      }
    }
  }

  private clearPendingInput(): void {
    if (this.inputTimer !== undefined) {
      clearTimeout(this.inputTimer);
      this.inputTimer = undefined;
    }
    this.pendingInput = "";
  }

  private emitReadResult(chunks: readonly ITerminalProcessOutputChunk[], commandEvents: readonly ITerminalProcessCommandStatusEvent[]): void {
    let outputSequence = this.nextSequence;
    let eventIndex = 0;
    const emitEventsThrough = (sequence: number): void => {
      while (eventIndex < commandEvents.length && commandEvents[eventIndex]!.afterOutputSequence <= sequence) {
        const event = commandEvents[eventIndex++]!;
        this._onDidChangeCommandStatus.fire({
          commandId: event.commandId,
          status: event.status,
          exitCode: event.exitCode ?? undefined,
        });
      }
    };
    emitEventsThrough(outputSequence);
    for (const chunk of chunks) {
      emitEventsThrough(chunk.sequence - 1);
      this._onDidWriteData.fire(decodeBase64(chunk.dataBase64));
      outputSequence = chunk.sequence;
      emitEventsThrough(outputSequence);
    }
    emitEventsThrough(Number.POSITIVE_INFINITY);
  }

  private setState(state: TerminalInstanceState): void {
    if (this._state === state || this.closed) return;
    this._state = state;
    this._onDidChangeState.fire(state);
  }
}

function decodeBase64(value: string): Uint8Array {
  const binary = atob(value);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }
  return bytes;
}

function delay(milliseconds: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}

function takeUtf8Prefix(value: string, maximumBytes: number): string {
  const encoder = new TextEncoder();
  if (encoder.encode(value).byteLength <= maximumBytes) return value;
  let lower = 1;
  let upper = value.length;
  while (lower < upper) {
    const middle = Math.ceil((lower + upper) / 2);
    if (encoder.encode(value.slice(0, middle)).byteLength <= maximumBytes) {
      lower = middle;
    } else {
      upper = middle - 1;
    }
  }
  if (lower < value.length && isHighSurrogate(value.charCodeAt(lower - 1))) {
    lower -= 1;
  }
  return value.slice(0, Math.max(1, lower));
}

function isHighSurrogate(codeUnit: number): boolean {
  return codeUnit >= 0xd800 && codeUnit <= 0xdbff;
}

function terminalProfileTitle(profile: ITerminalProfile): string {
  if (profile.profileId === "cmd" || profile.profileId === "command-prompt") {
    return "cmd";
  }
  return profile.title;
}
