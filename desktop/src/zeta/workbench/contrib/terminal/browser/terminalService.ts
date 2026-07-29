import { Emitter, type Event } from "../../../../base/common/event.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import type { AppServerConnectionState } from "../../../../platform/app-server/common/renderer-api.js";
import type { ITerminalBackend } from "./appServerTerminalBackend.js";
import type { ITerminalCreateOptions, ITerminalDimensions, ITerminalInstance, ITerminalProfile, ITerminalService, TerminalInstanceState } from "../common/terminal.js";

const POLL_DELAY_MILLIS = 35;
const INPUT_BATCH_DELAY_MILLIS = 8;
const INPUT_BATCH_CHARACTERS = 16_384;
const MAX_INPUT_BATCH_BYTES = 60 * 1024;
const MAX_READ_CHUNKS = 128;

/** Browser Workbench terminal owner backed by connection-scoped App Server PTYs. */
export class TerminalService extends DisposableOwner implements ITerminalService {
  readonly #backend: ITerminalBackend;
  readonly #instances: TerminalInstance[] = [];
  readonly #onDidCreateInstance = this.own(new Emitter<ITerminalInstance>());
  readonly #onDidDisposeInstance = this.own(new Emitter<ITerminalInstance>());
  readonly #onDidChangeActiveInstance = this.own(new Emitter<ITerminalInstance | undefined>());
  #activeInstance: TerminalInstance | undefined;
  #nextInstanceId = 1;
  #connectionState: AppServerConnectionState = "ready";
  #connectionRevision = 0;

  readonly onDidCreateInstance: Event<ITerminalInstance> = this.#onDidCreateInstance.event;
  readonly onDidDisposeInstance: Event<ITerminalInstance> = this.#onDidDisposeInstance.event;
  readonly onDidChangeActiveInstance: Event<ITerminalInstance | undefined> = this.#onDidChangeActiveInstance.event;

  constructor(backend: ITerminalBackend) {
    super();
    this.#backend = backend;
    this.own(backend.onConnectionState((state) => {
      this.#connectionRevision += 1;
      this.#setConnectionState(state);
    }));
    const connectionRevision = this.#connectionRevision;
    void backend.getConnectionState()
      .then((state) => {
        if (this.#connectionRevision === connectionRevision) this.#setConnectionState(state);
      })
      .catch(() => {
        if (this.#connectionRevision === connectionRevision) this.#setConnectionState("crashed");
      });
    this.defer(() => {
      for (const instance of [...this.#instances]) {
        void instance.close().catch(() => {});
      }
      this.#instances.length = 0;
      this.#activeInstance = undefined;
    });
  }

  get instances(): readonly ITerminalInstance[] {
    return this.#instances;
  }

  get activeInstance(): ITerminalInstance | undefined {
    return this.#activeInstance;
  }

  async getProfiles(): Promise<readonly ITerminalProfile[]> {
    const result = await this.#backend.listProfiles();
    return result.profiles;
  }

  async createTerminal(options: ITerminalCreateOptions): Promise<ITerminalInstance> {
    const created = await this.#backend.create({
      rows: options.dimensions.rows,
      cols: options.dimensions.cols,
      profile: options.profile,
    });
    const instanceNumber = this.#nextInstanceId++;
    const instance = this.own(new TerminalInstance(
      `terminal-instance-${instanceNumber}`,
      created.terminalId,
      `${created.profile.title} ${instanceNumber}`,
      created.profile,
      this.#backend,
      () => this.#removeInstance(instance),
    ));
    this.#instances.push(instance);
    this.#onDidCreateInstance.fire(instance);
    this.setActiveInstance(instance);
    instance.start();
    return instance;
  }

  async relaunchTerminal(instance: ITerminalInstance, dimensions: ITerminalDimensions): Promise<void> {
    if (!this.#instances.includes(instance as TerminalInstance)) {
      throw new Error("Terminal must belong to this TerminalService");
    }
    await (instance as TerminalInstance).relaunch(dimensions);
  }

  setActiveInstance(instance: ITerminalInstance | undefined): void {
    if (instance !== undefined && !this.#instances.includes(instance as TerminalInstance)) {
      throw new Error("Active terminal must belong to this TerminalService");
    }
    if (this.#activeInstance === instance) return;
    this.#activeInstance = instance as TerminalInstance | undefined;
    this.#onDidChangeActiveInstance.fire(instance);
  }

  async closeTerminal(instance: ITerminalInstance): Promise<void> {
    if (!this.#instances.includes(instance as TerminalInstance)) return;
    await instance.close();
  }

  #removeInstance(instance: TerminalInstance): void {
    const index = this.#instances.indexOf(instance);
    if (index < 0) return;
    this.#instances.splice(index, 1);
    this.#onDidDisposeInstance.fire(instance);
    if (this.#activeInstance === instance) {
      this.#activeInstance = this.#instances.at(-1);
      this.#onDidChangeActiveInstance.fire(this.#activeInstance);
    }
  }

  #setConnectionState(state: AppServerConnectionState): void {
    if (this.#connectionState === state) return;
    this.#connectionState = state;
    if (state === "ready") return;
    for (const instance of this.#instances) {
      instance.disconnect();
    }
  }
}

class TerminalInstance extends DisposableOwner implements ITerminalInstance {
  readonly #backend: ITerminalBackend;
  readonly #onClosed: () => void;
  readonly #onDidWriteData = this.own(new Emitter<Uint8Array>());
  readonly #onDidExit = this.own(new Emitter<number | undefined>());
  readonly #onDidChangeState = this.own(new Emitter<TerminalInstanceState>());
  #state: TerminalInstanceState = "running";
  #exitCode: number | undefined;
  #nextSequence = 0;
  #closed = false;
  #pendingInput = "";
  #inputTimer: ReturnType<typeof setTimeout> | undefined;
  #writeChain = Promise.resolve();
  #pendingDimensions: ITerminalDimensions | undefined;
  #resizeScheduled = false;
  #backendId: string;
  #profile: ITerminalProfile;
  #pollGeneration = 0;

  readonly onDidWriteData: Event<Uint8Array> = this.#onDidWriteData.event;
  readonly onDidExit: Event<number | undefined> = this.#onDidExit.event;
  readonly onDidChangeState: Event<TerminalInstanceState> = this.#onDidChangeState.event;

  constructor(
    readonly id: string,
    backendId: string,
    readonly title: string,
    profile: ITerminalProfile,
    backend: ITerminalBackend,
    onClosed: () => void,
  ) {
    super();
    this.#backendId = backendId;
    this.#profile = profile;
    this.#backend = backend;
    this.#onClosed = onClosed;
    this.defer(() => {
      if (this.#inputTimer !== undefined) clearTimeout(this.#inputTimer);
    });
  }

  get state(): TerminalInstanceState {
    return this.#state;
  }

  get exitCode(): number | undefined {
    return this.#exitCode;
  }

  get profile(): ITerminalProfile {
    return this.#profile;
  }

  start(): void {
    const generation = ++this.#pollGeneration;
    void this.#poll(generation);
  }

  write(data: string): void {
    if (this.#closed || this.#state !== "running" || data.length === 0) return;
    this.#pendingInput += data;
    if (this.#pendingInput.length >= INPUT_BATCH_CHARACTERS) {
      this.#flushInput();
      return;
    }
    if (this.#inputTimer !== undefined) return;
    this.#inputTimer = setTimeout(() => {
      this.#inputTimer = undefined;
      this.#flushInput();
    }, INPUT_BATCH_DELAY_MILLIS);
  }

  resize(dimensions: ITerminalDimensions): void {
    if (this.#closed || this.#state !== "running") return;
    this.#pendingDimensions = dimensions;
    if (this.#resizeScheduled) return;
    this.#resizeScheduled = true;
    queueMicrotask(() => {
      this.#resizeScheduled = false;
      const pending = this.#pendingDimensions;
      this.#pendingDimensions = undefined;
      if (!pending || this.#closed || this.#state !== "running") return;
      const generation = this.#pollGeneration;
      void this.#backend.resize({
        terminalId: this.#backendId,
        rows: pending.rows,
        cols: pending.cols,
      }).catch(() => {
        if (generation === this.#pollGeneration && this.#state === "running") {
          this.#setState("error");
        }
      });
    });
  }

  async close(): Promise<void> {
    if (this.#closed) return;
    this.#closed = true;
    if (this.#inputTimer !== undefined) {
      clearTimeout(this.#inputTimer);
      this.#inputTimer = undefined;
    }
    this.#pendingInput = "";
    this.#pollGeneration += 1;
    try {
      await this.#backend.close({ terminalId: this.#backendId });
    } finally {
      this.#onClosed();
      this.dispose();
    }
  }

  disconnect(): void {
    if (this.#closed || this.#state === "exited" || this.#state === "disconnected") return;
    this.#pollGeneration += 1;
    this.#clearPendingInput();
    this.#setState("disconnected");
    this.#onDidWriteData.fire(new TextEncoder().encode("\r\n[App Server disconnected; terminal process was lost]\r\n"));
  }

  async relaunch(dimensions: ITerminalDimensions): Promise<void> {
    if (this.#closed || this.#state === "running") return;
    await this.#backend.close({ terminalId: this.#backendId }).catch(() => {});
    try {
      const created = await this.#backend.create({
        rows: dimensions.rows,
        cols: dimensions.cols,
        profile: {
          type: "profile",
          profileId: this.#profile.profileId,
        },
      });
      this.#backendId = created.terminalId;
      this.#profile = created.profile;
      this.#nextSequence = 0;
      this.#exitCode = undefined;
      this.#onDidWriteData.fire(new TextEncoder().encode("\r\n[terminal relaunched]\r\n"));
      this.#setState("running");
      this.start();
    } catch (error) {
      this.#setState("error");
      throw error;
    }
  }

  #flushInput(): void {
    if (this.#inputTimer !== undefined) {
      clearTimeout(this.#inputTimer);
      this.#inputTimer = undefined;
    }
    if (this.#closed || this.#state !== "running" || this.#pendingInput.length === 0) return;
    const data = takeUtf8Prefix(this.#pendingInput, MAX_INPUT_BATCH_BYTES);
    this.#pendingInput = this.#pendingInput.slice(data.length);
    const generation = this.#pollGeneration;
    this.#writeChain = this.#writeChain
      .then(() => {
        if (generation !== this.#pollGeneration || this.#state !== "running") return;
        return this.#backend.write({ terminalId: this.#backendId, data });
      })
      .catch(() => {
        if (generation === this.#pollGeneration && this.#state === "running") {
          this.#setState("error");
        }
      })
      .then(() => {
        if (generation === this.#pollGeneration && this.#state === "running" && this.#pendingInput.length > 0) {
          this.#flushInput();
        }
      });
  }

  async #poll(generation: number): Promise<void> {
    while (!this.#closed && this.#state === "running" && generation === this.#pollGeneration) {
      try {
        const result = await this.#backend.read({
          terminalId: this.#backendId,
          afterSequence: this.#nextSequence,
          maxChunks: MAX_READ_CHUNKS,
        });
        if (this.#closed || this.#state !== "running" || generation !== this.#pollGeneration) return;
        if (result.outputGap) {
          this.#onDidWriteData.fire(new TextEncoder().encode("\r\n[terminal output truncated]\r\n"));
        }
        for (const chunk of result.chunks) {
          this.#onDidWriteData.fire(decodeBase64(chunk.dataBase64));
        }
        this.#nextSequence = result.nextSequence;
        if (result.exited) {
          this.#exitCode = result.exitCode ?? undefined;
          this.#setState("exited");
          this.#onDidExit.fire(this.#exitCode);
          return;
        }
        if (result.chunks.length === 0) await delay(POLL_DELAY_MILLIS);
      } catch {
        if (!this.#closed && generation === this.#pollGeneration) this.#setState("error");
        return;
      }
    }
  }

  #clearPendingInput(): void {
    if (this.#inputTimer !== undefined) {
      clearTimeout(this.#inputTimer);
      this.#inputTimer = undefined;
    }
    this.#pendingInput = "";
  }

  #setState(state: TerminalInstanceState): void {
    if (this.#state === state || this.#closed) return;
    this.#state = state;
    this.#onDidChangeState.fire(state);
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
