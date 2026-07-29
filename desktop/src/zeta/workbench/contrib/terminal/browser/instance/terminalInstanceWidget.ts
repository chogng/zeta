import "@xterm/xterm/css/xterm.css";
import { FitAddon } from "@xterm/addon-fit";
import { Terminal } from "@xterm/xterm";
import { DisposableOwner, toDisposable } from "../../../../../base/common/lifecycle.js";
import type { IThemeService } from "../../../../../platform/theme/common/themeService.js";
import type { ITerminalDimensions, ITerminalInstance } from "../../../../services/terminal/common/terminal.js";
import { terminalTheme } from "./terminalTheme.js";

/** One persistent xterm renderer bound to exactly one Terminal instance. */
export class TerminalInstanceWidget extends DisposableOwner {
  readonly element: HTMLDivElement;
  readonly #terminal: Terminal;
  readonly #fitAddon = new FitAddon();
  #visible = false;

  constructor(readonly instance: ITerminalInstance, ownerDocument: Document, themeService: IThemeService) {
    super();
    this.element = ownerDocument.createElement("div");
    this.element.className = "zeta-terminal-instance";
    this.element.hidden = true;
    this.#terminal = new Terminal({
      allowTransparency: false,
      cursorBlink: true,
      cursorStyle: "block",
      fontFamily: "var(--zeta-font-family-monospace, Consolas, 'Courier New', monospace)",
      fontSize: 13,
      scrollback: 5_000,
      theme: terminalTheme(themeService.getColorTheme()),
    });
    this.own(themeService.onDidColorThemeChange((theme) => {
      this.#terminal.options.theme = terminalTheme(theme);
    }));
    this.#terminal.loadAddon(this.#fitAddon);
    this.#terminal.open(this.element);
    this.defer(() => this.#terminal.dispose());
    const input = this.#terminal.onData((data) => this.instance.write(data));
    this.own(toDisposable(() => input.dispose()));
    this.own(instance.onDidWriteData((data) => this.#terminal.write(data)));
    this.own(instance.onDidExit((exitCode) => {
      this.#terminal.writeln("");
      this.#terminal.writeln(`[process exited with code ${exitCode ?? "unknown"}]`);
    }));
    this.own(instance.onDidChangeState((state) => {
      this.element.dataset.state = state;
      if (state === "error") {
        this.#terminal.writeln("");
        this.#terminal.writeln("[terminal operation failed]");
      }
    }));
    this.element.dataset.state = instance.state;
    const ResizeObserverConstructor = ownerDocument.defaultView?.ResizeObserver;
    if (ResizeObserverConstructor) {
      const observer = new ResizeObserverConstructor(() => this.fit());
      observer.observe(this.element);
      this.defer(() => observer.disconnect());
    }
  }

  setVisible(visible: boolean): void {
    if (this.#visible === visible) return;
    this.#visible = visible;
    this.element.hidden = !visible;
    if (visible) queueMicrotask(() => this.fit());
  }

  focus(): void {
    if (this.#visible) this.#terminal.focus();
  }

  fit(): void {
    if (!this.#visible) return;
    try {
      this.#fitAddon.fit();
    } catch {
      return;
    }
    this.instance.resize(this.dimensions());
  }

  dimensions(): ITerminalDimensions {
    return {
      rows: Math.min(512, Math.max(1, this.#terminal.rows)),
      cols: Math.min(512, Math.max(1, this.#terminal.cols)),
    };
  }
}
