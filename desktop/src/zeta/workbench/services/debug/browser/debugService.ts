import { Emitter, type Event } from "../../../../base/common/event.js";
import { DisposableOwner, DisposableSlot } from "../../../../base/common/lifecycle.js";
import { type URI } from "../../../../base/common/uri.js";
import { type IDebugAdapterProcessService } from "../../../../platform/debug/common/debugAdapterProcessService.js";
import { FileNotFoundError, type IFileService } from "../../../../platform/files/common/files.js";
import { type IWorkspaceContextService } from "../../../../platform/workspace/common/workspace.js";
import { DebugAdapterSession } from "./debugAdapterSession.js";
import { type IDebugBreakpoint, type IDebugConfiguration, type IDebugService, type IDebugSession } from "../common/debugService.js";
import { parseLaunchConfigurations } from "../common/launchConfiguration.js";
import { type ITerminalService } from "../../terminal/common/terminal.js";
import { runDebuggeeInTerminal } from "./debugTerminalLauncher.js";

/** Workspace Debug composition over generic DAP processes. */
export class DebugService extends DisposableOwner implements IDebugService {
  private readonly configurationsEmitter = this.own(new Emitter<readonly IDebugConfiguration[]>());
  private readonly breakpointsEmitter = this.own(new Emitter<readonly IDebugBreakpoint[]>());
  private readonly sessionEmitter = this.own(new Emitter<IDebugSession | undefined>());
  private readonly sessionSlot = this.own(new DisposableSlot<DebugAdapterSession>());
  private readonly sessionListenerSlot = this.own(new DisposableSlot());
  private currentConfigurations: readonly IDebugConfiguration[] = Object.freeze([]);
  private currentBreakpoints: readonly IDebugBreakpoint[] = Object.freeze([]);
  private refreshGeneration = 0;

  readonly onDidChangeConfigurations: Event<readonly IDebugConfiguration[]> = this.configurationsEmitter.event;
  readonly onDidChangeBreakpoints: Event<readonly IDebugBreakpoint[]> = this.breakpointsEmitter.event;
  readonly onDidChangeSession: Event<IDebugSession | undefined> = this.sessionEmitter.event;

  constructor(private readonly files: IFileService, private readonly workspace: IWorkspaceContextService, private readonly processes: IDebugAdapterProcessService | undefined, private readonly terminals: ITerminalService) {
    super();
    this.own(files.onDidChangeFiles(event => { if (event.resources === undefined || event.resources.some(resource => /\/\.vscode\/launch\.json$/i.test(resource.path))) void this.refresh().catch(reportError); }));
    this.own(workspace.onDidChangeWorkspace(() => { this.refreshGeneration += 1; this.setConfigurations(Object.freeze([])); void this.stop(); }));
  }

  get configurations() { return this.currentConfigurations; }
  get breakpoints() { return this.currentBreakpoints; }
  get session(): IDebugSession | undefined { return this.sessionSlot.value; }

  async refresh(): Promise<readonly IDebugConfiguration[]> {
    const generation = ++this.refreshGeneration;
    const root = this.workspace.getWorkspace().folders[0]?.uri;
    let configurations: readonly IDebugConfiguration[] = Object.freeze([]);
    if (root) {
      try { configurations = parseLaunchConfigurations((await this.files.readFile(childResource(root, ".vscode/launch.json"))).content); }
      catch (error) { if (!(error instanceof FileNotFoundError)) throw error; }
    }
    if (generation === this.refreshGeneration) this.setConfigurations(configurations);
    return this.currentConfigurations;
  }

  async start(configuration: IDebugConfiguration): Promise<IDebugSession> {
    if (!this.processes) throw new Error("This host does not provide the Code debug adapter capability");
    const current = this.currentConfigurations.find(candidate => candidate.id === configuration.id);
    if (!current) throw new Error("Debug configuration is no longer present in launch.json");
    await this.stop();
    const root = this.workspace.getWorkspace().folders[0]?.uri;
    if (!root) throw new Error("Debugging requires an open workspace folder");
    const workspaceFolder = root.scheme === "file" ? root.fsPath : decodeURIComponent(root.path);
    const session = await DebugAdapterSession.start({ configuration: current, processService: this.processes, breakpoints: () => this.currentBreakpoints, workspaceFolder, runInTerminal: value => runDebuggeeInTerminal(this.terminals, value), updateBreakpoints: updates => this.acceptBreakpointUpdates(updates) });
    this.sessionSlot.replace(session);
    this.sessionListenerSlot.replace(session.onDidChangeState(state => {
      this.sessionEmitter.fire(session);
      if (state === "terminated" || state === "error") queueMicrotask(() => { if (this.sessionSlot.value === session) { this.sessionListenerSlot.clear(); this.sessionSlot.clear(); this.sessionEmitter.fire(undefined); } });
    }));
    this.sessionEmitter.fire(session);
    return session;
  }

  async stop(): Promise<void> {
    const session = this.sessionSlot.value;
    if (!session) return;
    await session.disconnect();
    this.sessionListenerSlot.clear();
    if (this.sessionSlot.value === session) this.sessionSlot.clear();
    this.sessionEmitter.fire(undefined);
  }

  toggleBreakpoint(resource: URI, lineNumber: number): void {
    if (!Number.isSafeInteger(lineNumber) || lineNumber <= 0) throw new RangeError("Breakpoint line number must be positive");
    const existing = this.currentBreakpoints.find(breakpoint => breakpoint.resource.toString() === resource.toString() && breakpoint.lineNumber === lineNumber);
    this.currentBreakpoints = existing ? Object.freeze(this.currentBreakpoints.filter(breakpoint => breakpoint !== existing)) : Object.freeze([...this.currentBreakpoints, Object.freeze({ id: `${resource.toString()}:${lineNumber}`, resource, lineNumber, enabled: true, verified: false })].sort(compareBreakpoints));
    this.breakpointsEmitter.fire(this.currentBreakpoints);
    const session = this.sessionSlot.value;
    if (session) void session.syncBreakpoints().catch(reportError);
  }

  removeBreakpoint(id: string): void {
    const next = this.currentBreakpoints.filter(breakpoint => breakpoint.id !== id);
    if (next.length === this.currentBreakpoints.length) return;
    this.currentBreakpoints = Object.freeze(next);
    this.breakpointsEmitter.fire(this.currentBreakpoints);
    const session = this.sessionSlot.value;
    if (session) void session.syncBreakpoints().catch(reportError);
  }

  private setConfigurations(configurations: readonly IDebugConfiguration[]): void {
    if (JSON.stringify(configurations) === JSON.stringify(this.currentConfigurations)) return;
    this.currentConfigurations = configurations;
    this.configurationsEmitter.fire(configurations);
  }

  private acceptBreakpointUpdates(updates: readonly { readonly id: string; readonly verified: boolean; readonly message?: string }[]): void {
    if (updates.length === 0) return;
    const byId = new Map(updates.map(update => [update.id, update]));
    this.currentBreakpoints = Object.freeze(this.currentBreakpoints.map(breakpoint => {
      const update = byId.get(breakpoint.id);
      return update ? Object.freeze({ ...breakpoint, verified: update.verified, ...(update.message === undefined ? {} : { message: update.message }) }) : breakpoint;
    }));
    this.breakpointsEmitter.fire(this.currentBreakpoints);
  }
}

function childResource(root: URI, relativePath: string): URI { const base = root.path.endsWith("/") ? root.path.slice(0, -1) : root.path; return root.withPath(`${base}/${relativePath.split("/").map(encodeURIComponent).join("/")}`); }
function compareBreakpoints(left: IDebugBreakpoint, right: IDebugBreakpoint): number { return left.resource.toString().localeCompare(right.resource.toString()) || left.lineNumber - right.lineNumber; }
function reportError(error: unknown): void { console.error("Debug service operation failed", error); }
