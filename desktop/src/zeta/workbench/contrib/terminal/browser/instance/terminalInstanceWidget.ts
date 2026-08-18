import "@xterm/xterm/css/xterm.css";
import { FitAddon } from "@xterm/addon-fit";
import { Terminal, type IDecoration } from "@xterm/xterm";
import { DisposableOwner, toDisposable } from "../../../../../base/common/lifecycle.js";
import type { IThemeService } from "../../../../../platform/theme/common/themeService.js";
import type { ITerminalCommandStatusEvent, ITerminalDimensions, ITerminalInstance } from "../../../../services/terminal/common/terminal.js";
import { terminalTheme } from "./terminalTheme.js";
import { h } from "../../../../../base/browser/dom.js";
import { observeResize } from "../../../../../base/browser/observer.js";

/** One persistent xterm renderer bound to exactly one Terminal instance. */
export class TerminalInstanceWidget extends DisposableOwner {
  readonly element: HTMLDivElement;
  private readonly terminal: Terminal;
  private readonly fitAddon = new FitAddon();
  private readonly commandDecorations = new Map<string, TerminalCommandDecoration>();
  private visible = false;

  constructor(container: HTMLElement, readonly instance: ITerminalInstance, themeService: IThemeService) {
    super();
    this.element = h(container.ownerDocument, "div");
    this.element.className = "zeta-terminal-instance";
    this.element.hidden = true;
    container.append(this.element);
    this.terminal = new Terminal({
      allowProposedApi: true,
      allowTransparency: false,
      cursorBlink: true,
      cursorStyle: "block",
      fontFamily: "var(--zeta-font-family-monospace, Consolas, 'Courier New', monospace)",
      fontSize: 13,
      scrollback: 5_000,
      theme: terminalTheme(themeService.getColorTheme()),
    });
    this.own(themeService.onDidColorThemeChange((theme) => {
      this.terminal.options.theme = terminalTheme(theme);
    }));
    this.terminal.loadAddon(this.fitAddon);
    this.terminal.open(this.element);
    this.defer(() => this.terminal.dispose());
    const input = this.terminal.onData((data) => this.instance.write(data));
    this.own(toDisposable(() => input.dispose()));
    this.own(instance.onDidWriteData((data) => this.terminal.write(data)));
    this.own(instance.onDidChangeCommandStatus((event) => this.renderCommandStatus(event)));
    this.own(instance.onDidExit((exitCode) => {
      this.terminal.writeln("");
      this.terminal.writeln(`[process exited with code ${exitCode ?? "unknown"}]`);
    }));
    this.own(instance.onDidChangeState((state) => {
      this.element.dataset.state = state;
      if (state === "error") {
        this.terminal.writeln("");
        this.terminal.writeln("[terminal operation failed]");
      }
    }));
    this.element.dataset.state = instance.state;
    this.own(observeResize(this.element, () => this.fit()));
  }

  setVisible(visible: boolean): void {
    if (this.visible === visible) return;
    this.visible = visible;
    this.element.hidden = !visible;
    if (visible) queueMicrotask(() => this.fit());
  }

  focus(): void {
    if (this.visible) this.terminal.focus();
  }

  clear(): void {
    this.terminal.clear();
  }

  fit(): void {
    if (!this.visible) return;
    try {
      this.fitAddon.fit();
    } catch {
      return;
    }
    this.instance.resize(this.dimensions());
  }

  dimensions(): ITerminalDimensions {
    return {
      rows: Math.min(512, Math.max(1, this.terminal.rows)),
      cols: Math.min(512, Math.max(1, this.terminal.cols)),
    };
  }

  private renderCommandStatus(event: ITerminalCommandStatusEvent): void {
    let item = this.commandDecorations.get(event.commandId);
    if (!item) {
      const marker = this.terminal.registerMarker(0);
      const decoration = this.terminal.registerDecoration({ marker, width: 1, layer: "top" });
      if (!decoration) return;
      item = { event, decoration };
      this.commandDecorations.set(event.commandId, item);
      const renderListener = decoration.onRender((element) => this.presentCommandStatus(element, item!));
      const disposeListener = decoration.onDispose(() => this.commandDecorations.delete(event.commandId));
      this.own(toDisposable(() => renderListener.dispose()));
      this.own(toDisposable(() => disposeListener.dispose()));
      this.own(toDisposable(() => decoration.dispose()));
    } else {
      item.event = event;
    }
    if (item.decoration.element) this.presentCommandStatus(item.decoration.element, item);
  }

  private presentCommandStatus(element: HTMLElement, item: TerminalCommandDecoration): void {
    const { status, exitCode } = item.event;
    element.classList.remove("running", "completed", "succeeded", "failed", "canceled");
    element.classList.add("zeta-terminal-command-status", status);
    element.dataset.commandStatus = status;
    const label = terminalCommandStatusLabel(status, exitCode);
    element.setAttribute("role", "img");
    element.setAttribute("aria-label", label);
    element.title = label;
  }
}

interface TerminalCommandDecoration {
  event: ITerminalCommandStatusEvent;
  readonly decoration: IDecoration;
}

function terminalCommandStatusLabel(status: ITerminalCommandStatusEvent["status"], exitCode: number | undefined): string {
  switch (status) {
    case "running": return "Command is running";
    case "completed": return "Command completed; exit code unavailable";
    case "succeeded": return "Command completed successfully";
    case "failed": return exitCode === undefined ? "Command failed" : `Command failed with exit code ${exitCode}`;
    case "canceled": return "Command was canceled";
  }
}
