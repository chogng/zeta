import { addDisposableListener } from "../../../../base/browser/dom.js";
import { URI } from "../../../../base/common/uri.js";
import { DisposableSlot } from "../../../../base/common/lifecycle.js";
import { TextPosition, TextRange } from "../../../../editor/common/core/text.js";
import { ViewPane, type IViewPaneOptions } from "../../../browser/parts/views/viewPane.js";
import { type IEditorPart } from "../../../browser/parts/editor/editorPart.js";
import { type IDebugBreakpoint, type IDebugConfiguration, type IDebugScope, type IDebugService, type IDebugStackFrame, type IDebugVariable } from "../../../services/debug/common/debugService.js";

/** Code Debug sidebar with launch, control, stack, variable, output, and breakpoint state. */
export class DebugViewPane extends ViewPane {
  private readonly configurationsElement: HTMLSelectElement;
  private readonly statusElement: HTMLDivElement;
  private readonly stackElement: HTMLUListElement;
  private readonly variablesElement: HTMLUListElement;
  private readonly breakpointsElement: HTMLUListElement;
  private readonly outputElement: HTMLPreElement;
  private frames: readonly IDebugStackFrame[] = [];
  private scopes: readonly IDebugScope[] = [];
  private variables: readonly IDebugVariable[] = [];
  private output = "";
  private error: string | undefined;
  private readonly outputSubscription = this.own(new DisposableSlot());

  constructor(options: IViewPaneOptions, private readonly debug: IDebugService, private readonly editor: IEditorPart) {
    super(options);
    this.contentElement.classList.add("zeta-debug");
    const controls = options.ownerDocument.createElement("div");
    controls.className = "zeta-debug-controls";
    this.configurationsElement = options.ownerDocument.createElement("select");
    this.configurationsElement.setAttribute("aria-label", "Debug configuration");
    controls.append(this.configurationsElement, ...[button(options.ownerDocument, "Start", "start"), button(options.ownerDocument, "Continue", "continue"), button(options.ownerDocument, "Pause", "pause"), button(options.ownerDocument, "Over", "stepOver"), button(options.ownerDocument, "Into", "stepInto"), button(options.ownerDocument, "Out", "stepOut"), button(options.ownerDocument, "Stop", "stop")]);
    this.statusElement = options.ownerDocument.createElement("div");
    this.statusElement.className = "zeta-debug-status";
    this.statusElement.setAttribute("role", "status");
    this.stackElement = section(options.ownerDocument, "Call Stack", "zeta-debug-stack");
    this.variablesElement = section(options.ownerDocument, "Variables", "zeta-debug-variables");
    this.breakpointsElement = section(options.ownerDocument, "Breakpoints", "zeta-debug-breakpoints");
    this.outputElement = options.ownerDocument.createElement("pre");
    this.outputElement.className = "zeta-debug-output";
    this.outputElement.setAttribute("aria-label", "Debug console output");
    this.contentElement.append(controls, this.statusElement, this.stackElement, this.variablesElement, this.breakpointsElement, this.outputElement);
    this.own(addDisposableListener(controls, "click", event => this.control(event)));
    this.own(addDisposableListener(this.stackElement, "click", event => this.activateFrame(event)));
    this.own(addDisposableListener(this.variablesElement, "click", event => this.expandVariable(event)));
    this.own(addDisposableListener(this.breakpointsElement, "click", event => this.activateBreakpoint(event)));
    this.own(debug.onDidChangeConfigurations(() => this.render()));
    this.own(debug.onDidChangeBreakpoints(() => this.render()));
    this.own(debug.onDidChangeSession(session => { this.frames = []; this.scopes = []; this.variables = []; this.outputSubscription.replace(session?.onDidOutput(output => { this.output = (this.output + output).slice(-128_000); this.render(); })); this.render(); if (session?.state === "stopped") void this.refreshStack(); }));
    this.render();
    void debug.refresh().catch(error => { this.error = message(error); this.render(); });
  }

  private control(event: Event): void {
    const target = event.target instanceof this.element.ownerDocument.defaultView!.Element ? event.target.closest<HTMLButtonElement>("button[data-operation]") : null;
    const operation = target?.dataset.operation;
    if (!operation) return;
    this.error = undefined;
    const session = this.debug.session;
    const action = operation === "start" ? this.startSelected() : operation === "stop" ? this.debug.stop() : session && operation in session ? (session[operation as "continue" | "pause" | "stepOver" | "stepInto" | "stepOut"] as () => Promise<void>).call(session) : Promise.resolve();
    void action.catch(error => { this.error = message(error); this.render(); });
  }

  private async startSelected(): Promise<void> {
    const configuration = this.debug.configurations.find(candidate => candidate.id === this.configurationsElement.value) ?? this.debug.configurations[0];
    if (!configuration) throw new Error("No debug configuration found in .vscode/launch.json");
    this.output = "";
    await this.debug.start(configuration);
  }

  private async refreshStack(): Promise<void> {
    const session = this.debug.session;
    if (!session || session.state !== "stopped") return;
    try { this.frames = await session.stackTrace(); this.scopes = this.frames[0] ? await session.scopes(this.frames[0].id) : []; this.variables = this.scopes[0] ? await session.variables(this.scopes[0].variablesReference) : []; }
    catch (error) { this.error = message(error); }
    this.render();
  }

  private activateFrame(event: Event): void {
    const index = indexFromEvent(event, ".zeta-debug-frame", "frameIndex", this.element.ownerDocument);
    const frame = index === undefined ? undefined : this.frames[index];
    if (!frame) return;
    const session = this.debug.session;
    void (async () => {
      if (frame.source?.path) { const position = TextPosition.at(frame.lineNumber - 1, Math.max(0, frame.columnNumber - 1)); await this.editor.openEditor({ resource: URI.file(frame.source.path), label: frame.source.name }, { selection: TextRange.emptyAt(position) }); }
      if (session) { this.scopes = await session.scopes(frame.id); this.variables = this.scopes[0] ? await session.variables(this.scopes[0].variablesReference) : []; this.render(); }
    })().catch(error => { this.error = message(error); this.render(); });
  }

  private expandVariable(event: Event): void {
    const index = indexFromEvent(event, ".zeta-debug-variable", "variableIndex", this.element.ownerDocument);
    const variable = index === undefined ? undefined : this.variables[index];
    const session = this.debug.session;
    if (!variable || !session || variable.variablesReference <= 0) return;
    void session.variables(variable.variablesReference).then(variables => { this.variables = variables; this.render(); }, error => { this.error = message(error); this.render(); });
  }

  private activateBreakpoint(event: Event): void {
    const index = indexFromEvent(event, ".zeta-debug-breakpoint", "breakpointIndex", this.element.ownerDocument);
    const breakpoint = index === undefined ? undefined : this.debug.breakpoints[index];
    const remove = event.target instanceof this.element.ownerDocument.defaultView!.Element && Boolean(event.target.closest(".zeta-debug-breakpoint-remove"));
    if (!breakpoint) return;
    if (remove) this.debug.removeBreakpoint(breakpoint.id);
    else void this.editor.openEditor({ resource: breakpoint.resource }, { selection: lineSelection(breakpoint.lineNumber) }).catch(error => { this.error = message(error); this.render(); });
  }

  private render(): void {
    const selected = this.configurationsElement.value;
    this.configurationsElement.replaceChildren(...this.debug.configurations.map(configuration => option(this.element.ownerDocument, configuration)));
    if (this.debug.configurations.some(configuration => configuration.id === selected)) this.configurationsElement.value = selected;
    const session = this.debug.session;
    this.statusElement.textContent = this.error ?? (!session ? `${this.debug.configurations.length} debug configuration${this.debug.configurations.length === 1 ? "" : "s"}.` : `${session.configuration.name}: ${session.state}${session.reason ? ` (${session.reason})` : ""}`);
    this.stackElement.replaceChildren(heading(this.element.ownerDocument, "Call Stack"), ...this.frames.map((frame, index) => itemButton(this.element.ownerDocument, `${frame.name}  ${frame.source?.name ?? frame.source?.path ?? ""}:${frame.lineNumber}`, "zeta-debug-frame", "frameIndex", index)));
    this.variablesElement.replaceChildren(heading(this.element.ownerDocument, "Variables"), ...this.variables.map((variable, index) => itemButton(this.element.ownerDocument, `${variable.name} = ${variable.value}${variable.type ? ` : ${variable.type}` : ""}`, "zeta-debug-variable", "variableIndex", index)));
    this.breakpointsElement.replaceChildren(heading(this.element.ownerDocument, "Breakpoints"), ...this.debug.breakpoints.map((breakpoint, index) => breakpointItem(this.element.ownerDocument, breakpoint, index)));
    this.outputElement.textContent = this.output;
  }
}

function button(document: Document, label: string, operation: string): HTMLButtonElement { const element = document.createElement("button"); element.type = "button"; element.textContent = label; element.dataset.operation = operation; return element; }
function option(document: Document, configuration: IDebugConfiguration): HTMLOptionElement { const element = document.createElement("option"); element.value = configuration.id; element.textContent = configuration.name; return element; }
function section(document: Document, label: string, className: string): HTMLUListElement { const element = document.createElement("ul"); element.className = `zeta-debug-section ${className}`; element.setAttribute("aria-label", label); return element; }
function heading(document: Document, label: string): HTMLLIElement { const element = document.createElement("li"); element.className = "zeta-debug-section-heading"; element.textContent = label; return element; }
function itemButton(document: Document, label: string, className: string, dataName: string, index: number): HTMLLIElement { const item = document.createElement("li"); const action = document.createElement("button"); action.type = "button"; action.className = className; action.textContent = label; action.dataset[dataName] = String(index); item.append(action); return item; }
function breakpointItem(document: Document, breakpoint: IDebugBreakpoint, index: number): HTMLLIElement { const item = itemButton(document, `${breakpoint.resource.path.split("/").at(-1)}:${breakpoint.lineNumber}`, "zeta-debug-breakpoint", "breakpointIndex", index); const remove = document.createElement("button"); remove.type = "button"; remove.className = "zeta-debug-breakpoint-remove"; remove.textContent = "Remove"; item.append(remove); return item; }
function indexFromEvent(event: Event, selector: string, dataName: string, document: Document): number | undefined { const target = event.target instanceof document.defaultView!.Element ? event.target.closest<HTMLElement>(selector) : null; const index = Number(target?.dataset[dataName]); return Number.isSafeInteger(index) ? index : undefined; }
function lineSelection(lineNumber: number): TextRange { return TextRange.emptyAt(TextPosition.at(lineNumber - 1, 0)); }
function message(error: unknown): string { return error instanceof Error ? error.message : String(error); }
